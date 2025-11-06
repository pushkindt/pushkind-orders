use std::sync::Arc;

use chrono::NaiveDateTime;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use serde::{Deserialize, Serialize};

use crate::domain::{
    category::{Category, CategoryTreeQuery},
    price_level::PriceLevelListQuery,
    product::{Product, ProductListQuery},
    tag::{Tag, TagListQuery},
};
use crate::repository::{CategoryReader, PriceLevelReader, ProductReader, TagReader};
use crate::services::{ServiceError, ServiceResult};

/// Trait implemented by repositories that expose read access required by the
/// storefront service layer.
pub trait StoreClientRepository:
    CategoryReader + ProductReader + TagReader + PriceLevelReader + Send + Sync
{
}

impl<T> StoreClientRepository for T where
    T: CategoryReader + ProductReader + TagReader + PriceLevelReader + Send + Sync
{
}

/// Type alias for a trait object that satisfies [`StoreClientRepository`].
pub type DynStoreClientRepository = dyn StoreClientRepository;

/// Convenience alias used by handlers when storing an authenticated store
/// client context inside the Actix request extensions.
pub type StoreClientHandle = StoreClientContext<DynStoreClientRepository>;

/// Context captured when a storefront request is associated with an optional
/// authenticated client.
pub struct StoreClientContext<R: ?Sized> {
    repository: Arc<R>,
}

impl<R: ?Sized> StoreClientContext<R> {
    /// Construct a new context backed by the supplied repository handle.
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Access the repository tied to this client context.
    pub fn repository(&self) -> &R {
        self.repository.as_ref()
    }
}

impl<R: ?Sized> Clone for StoreClientContext<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

/// Minimal representation of a category exposed to the storefront.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreCategory {
    /// Identifier of the category.
    pub id: i32,
    /// Parent identifier when the category belongs to a tree.
    pub parent_id: Option<i32>,
    /// Name displayed to users.
    pub name: String,
    /// Optional descriptive text.
    pub description: Option<String>,
    // Optional image_url serialized as imageUrl
    pub image_url: Option<String>,
}

impl From<Category> for StoreCategory {
    fn from(value: Category) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent_id,
            name: value.name,
            description: value.description,
            image_url: value.image_url,
        }
    }
}

/// Minimal representation of a tag exposed to the storefront.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreTag {
    /// Identifier of the tag.
    pub id: i32,
    /// Name displayed to users.
    pub name: String,
}

impl From<Tag> for StoreTag {
    fn from(value: Tag) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

/// Optional filters that can be applied when listing store categories.
#[derive(Debug, Clone, Default)]
pub struct StoreCategoryFilters {
    /// Only include categories belonging to this parent identifier.
    pub parent_id: Option<i32>,
}

/// Product payload formatted for storefront consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreProduct {
    /// Identifier of the product.
    pub id: i32,
    /// Optional category identifier used for grouping.
    pub category_id: Option<i32>,
    /// Name displayed to users.
    pub name: String,
    /// Optional stock keeping unit identifier.
    pub sku: Option<String>,
    /// Optional longer description.
    pub description: Option<String>,
    /// Optional unit of measure.
    pub units: Option<String>,
    /// ISO 4217 currency code associated with the product.
    pub currency: String,
    /// Price in smallest currency unit for the hub default price level, when configured.
    pub price_cents: Option<i32>,
    /// Tags attached to the product.
    pub tags: Vec<StoreTag>,
    /// Image URLs attached to the product.
    pub image_urls: Vec<String>,
    /// Timestamp representing when the product was last updated.
    pub updated_at: NaiveDateTime,
}

impl StoreProduct {
    fn from_domain(value: Product, default_price_level_id: Option<i32>) -> Self {
        let Product {
            id,
            hub_id: _,
            name,
            sku,
            description,
            units,
            currency,
            is_archived: _,
            category_id,
            price_levels,
            tags,
            image_urls,
            created_at: _,
            updated_at,
        } = value;

        let price_cents = default_price_level_id.and_then(|default_id| {
            price_levels
                .iter()
                .find(|rate| rate.price_level_id == default_id)
                .map(|rate| rate.price_cents)
        });

        Self {
            id,
            category_id,
            name,
            sku,
            description,
            units,
            currency,
            price_cents,
            tags: tags.into_iter().map(StoreTag::from).collect(),
            image_urls,
            updated_at,
        }
    }
}

impl From<Product> for StoreProduct {
    fn from(value: Product) -> Self {
        Self::from_domain(value, None)
    }
}

/// Optional filters that can be applied when listing store products.
#[derive(Debug, Clone, Default)]
pub struct StoreProductFilters {
    /// Only include products belonging to this category.
    pub category_id: Option<i32>,
    /// Filter products by a search term applied to the name and description.
    pub search: Option<String>,
    /// Fetch a specific page of products.
    pub page: Option<usize>,
}

impl StoreProductFilters {
    fn into_query(self, hub_id: i32) -> ProductListQuery {
        let mut query = ProductListQuery::new(hub_id);

        query = match self.category_id {
            Some(category_id) => query.with_category_id(category_id),
            None => query.only_without_category(),
        };

        if let Some(search) = self
            .search
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            query = query.search(search);
        }

        if let Some(page) = self.page.filter(|page| *page > 0) {
            query = query.paginate(page, DEFAULT_ITEMS_PER_PAGE);
        }

        query
    }
}

fn resolve_default_price_level_id<R>(repo: &R, hub_id: i32) -> ServiceResult<Option<i32>>
where
    R: PriceLevelReader + ?Sized,
{
    let (_, price_levels) = repo
        .list_price_levels(PriceLevelListQuery::new(hub_id))
        .map_err(ServiceError::from)?;

    Ok(price_levels
        .into_iter()
        .find(|level| level.is_default)
        .map(|level| level.id))
}

/// Load categories available to a storefront for the provided hub.
pub fn load_store_categories<R>(
    repo: &R,
    hub_id: i32,
    filters: StoreCategoryFilters,
    store_client: Option<&StoreClientHandle>,
) -> ServiceResult<Vec<StoreCategory>>
where
    R: CategoryReader + ?Sized,
{
    let query = CategoryTreeQuery::new(hub_id);
    let categories = match store_client {
        Some(client) => {
            client
                .repository()
                .list_categories(query)
                .map_err(ServiceError::from)?
                .1
        }
        None => repo.list_categories(query).map_err(ServiceError::from)?.1,
    };

    let parent_id = filters.parent_id;
    let filtered = categories
        .into_iter()
        .filter(|category| !category.is_archived)
        .filter(|category| match parent_id {
            Some(parent_id) => category.parent_id == Some(parent_id),
            None => category.parent_id.is_none(),
        })
        .map(StoreCategory::from)
        .collect();

    Ok(filtered)
}

/// Load products available to a storefront for the provided hub.
pub fn load_store_products<R>(
    repo: &R,
    hub_id: i32,
    filters: StoreProductFilters,
    store_client: Option<&StoreClientHandle>,
) -> ServiceResult<Vec<StoreProduct>>
where
    R: ProductReader + PriceLevelReader + ?Sized,
{
    let default_price_level_id = match store_client {
        Some(client) => resolve_default_price_level_id(client.repository(), hub_id)?,
        None => resolve_default_price_level_id(repo, hub_id)?,
    };

    let products = match store_client {
        Some(client) => {
            client
                .repository()
                .list_products(filters.clone().into_query(hub_id))
                .map_err(ServiceError::from)?
                .1
        }
        None => {
            repo.list_products(filters.into_query(hub_id))
                .map_err(ServiceError::from)?
                .1
        }
    };

    let filtered = products
        .into_iter()
        .filter(|product| !product.is_archived)
        .map(|product| StoreProduct::from_domain(product, default_price_level_id))
        .collect();

    Ok(filtered)
}

/// Load tags available to a storefront for the provided hub.
pub fn load_store_tags<R>(
    repo: &R,
    hub_id: i32,
    store_client: Option<&StoreClientHandle>,
) -> ServiceResult<Vec<StoreTag>>
where
    R: TagReader + ?Sized,
{
    let tags = match store_client {
        Some(client) => {
            client
                .repository()
                .list_tags(TagListQuery::new(hub_id))
                .map_err(ServiceError::from)?
                .1
        }
        None => {
            repo.list_tags(TagListQuery::new(hub_id))
                .map_err(ServiceError::from)?
                .1
        }
    };

    let mut formatted: Vec<StoreTag> = tags.into_iter().map(StoreTag::from).collect();
    formatted.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        price_level::{PriceLevel, PriceLevelListQuery},
        product::ProductListQuery,
        product_price_level::ProductPriceLevelRate,
    };
    use crate::repository::mock::{MockCategoryReader, MockPriceLevelReader, MockProductReader};
    use pushkind_common::repository::errors::RepositoryResult;

    fn sample_timestamp() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    struct MockStoreProductRepo {
        product_reader: MockProductReader,
        price_level_reader: MockPriceLevelReader,
    }

    impl MockStoreProductRepo {
        fn new(
            product_reader: MockProductReader,
            price_level_reader: MockPriceLevelReader,
        ) -> Self {
            Self {
                product_reader,
                price_level_reader,
            }
        }
    }

    impl ProductReader for MockStoreProductRepo {
        fn get_product_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Product>> {
            self.product_reader.get_product_by_id(id, hub_id)
        }

        fn list_products(
            &self,
            query: ProductListQuery,
        ) -> RepositoryResult<(usize, Vec<Product>)> {
            self.product_reader.list_products(query)
        }
    }

    impl PriceLevelReader for MockStoreProductRepo {
        fn get_price_level_by_id(
            &self,
            id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Option<PriceLevel>> {
            self.price_level_reader.get_price_level_by_id(id, hub_id)
        }

        fn list_price_levels(
            &self,
            query: PriceLevelListQuery,
        ) -> RepositoryResult<(usize, Vec<PriceLevel>)> {
            self.price_level_reader.list_price_levels(query)
        }
    }

    #[test]
    fn load_categories_filters_archived_items() {
        let mut repo = MockCategoryReader::new();
        let categories = vec![
            Category {
                id: 1,
                hub_id: 1,
                parent_id: None,
                name: "Coffee".to_string(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
            Category {
                id: 2,
                hub_id: 1,
                parent_id: None,
                name: "Archived".to_string(),
                description: None,
                is_archived: true,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
        ];

        repo.expect_list_categories()
            .withf(|query| query.hub_id == 1 && !query.include_archived)
            .return_once(move |_| Ok((2, categories.clone())));

        let result = load_store_categories(&repo, 1, StoreCategoryFilters::default(), None)
            .expect("load categories");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "Coffee");
    }

    #[test]
    fn load_categories_filters_by_parent_id() {
        let mut repo = MockCategoryReader::new();
        let categories = vec![
            Category {
                id: 1,
                hub_id: 1,
                parent_id: None,
                name: "Root".to_string(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
            Category {
                id: 2,
                hub_id: 1,
                parent_id: Some(1),
                name: "Child".to_string(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
        ];

        let categories_clone = categories.clone();
        repo.expect_list_categories()
            .withf(|query| query.hub_id == 1 && !query.include_archived)
            .times(2)
            .returning(move |_| Ok((2, categories_clone.clone())));

        let roots = load_store_categories(&repo, 1, StoreCategoryFilters::default(), None)
            .expect("load root categories");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, 1);

        let children =
            load_store_categories(&repo, 1, StoreCategoryFilters { parent_id: Some(1) }, None)
                .expect("load child categories");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, 2);
    }

    #[test]
    fn load_products_includes_tags() {
        let mut product_reader = MockProductReader::new();
        let products = vec![
            Product {
                id: 1,
                hub_id: 1,
                name: "Coffee".to_string(),
                sku: Some("SKU-1".to_string()),
                description: Some("Fresh beans".to_string()),
                units: Some("kg".to_string()),
                currency: "USD".to_string(),
                is_archived: false,
                category_id: Some(1),
                price_levels: vec![ProductPriceLevelRate {
                    id: 1,
                    product_id: 1,
                    price_level_id: 10,
                    price_cents: 1_299,
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                }],
                tags: vec![Tag {
                    id: 1,
                    hub_id: 1,
                    name: "Organic".to_string(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                }],
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_urls: vec!["https://example.com/coffee.png".to_string()],
            },
            Product {
                id: 2,
                hub_id: 1,
                name: "Retired".to_string(),
                sku: None,
                description: None,
                units: None,
                currency: "USD".to_string(),
                is_archived: true,
                category_id: None,
                price_levels: Vec::new(),
                tags: Vec::new(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_urls: Vec::new(),
            },
        ];

        let products_clone = products.clone();
        product_reader
            .expect_list_products()
            .withf(|query| {
                query.hub_id == 1
                    && !query.include_archived
                    && query.only_without_category
                    && query.category_id.is_none()
            })
            .return_once(move |_| Ok((2, products_clone)));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![PriceLevel {
            id: 10,
            hub_id: 1,
            name: "Default".to_string(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            is_default: true,
        }];

        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id == 1)
            .return_once(move |_| Ok((1, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let result = load_store_products(&repo, 1, StoreProductFilters::default(), None)
            .expect("load products");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "Coffee");
        assert_eq!(result[0].tags.len(), 1);
        assert_eq!(result[0].tags[0].name, "Organic");
        assert_eq!(result[0].price_cents, Some(1_299));
        assert_eq!(
            result[0].image_urls,
            vec!["https://example.com/coffee.png".to_string()]
        );
    }

    #[test]
    fn load_store_products_defaults_to_uncategorized() {
        let mut product_reader = MockProductReader::new();
        let uncategorized = Product {
            id: 1,
            hub_id: 1,
            name: "Andromeda".to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "USD".to_string(),
            is_archived: false,
            category_id: None,
            price_levels: Vec::new(),
            tags: Vec::new(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: Vec::new(),
        };

        product_reader
            .expect_list_products()
            .withf(|query| {
                query.hub_id == 1
                    && query.category_id.is_none()
                    && query.only_without_category
                    && !query.include_archived
            })
            .return_once(move |_| Ok((1, vec![uncategorized.clone()])));

        let mut price_level_reader = MockPriceLevelReader::new();
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id == 1)
            .return_once(|_| Ok((0, Vec::new())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let result = load_store_products(&repo, 1, StoreProductFilters::default(), None)
            .expect("load products");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].category_id, None);
        assert_eq!(result[0].price_cents, None);
    }

    #[test]
    fn load_store_products_applies_filters() {
        let mut product_reader = MockProductReader::new();

        product_reader
            .expect_list_products()
            .withf(|query| {
                query.hub_id == 1
                    && query.category_id == Some(3)
                    && !query.only_without_category
                    && query.search.as_deref() == Some("coffee")
                    && matches!(
                        query.pagination.as_ref(),
                        Some(pagination)
                            if pagination.page == 2
                                && pagination.per_page == DEFAULT_ITEMS_PER_PAGE
                    )
            })
            .return_once(|_| Ok((0, Vec::new())));

        let mut price_level_reader = MockPriceLevelReader::new();
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id == 1)
            .return_once(|_| Ok((0, Vec::new())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let filters = StoreProductFilters {
            category_id: Some(3),
            search: Some(" coffee ".to_string()),
            page: Some(2),
        };

        let result = load_store_products(&repo, 1, filters, None).expect("load products");
        assert!(result.is_empty());
    }

    #[test]
    fn prefers_store_client_context_when_present() {
        struct PanicRepo;

        impl CategoryReader for PanicRepo {
            fn list_categories(
                &self,
                _query: CategoryTreeQuery,
            ) -> RepositoryResult<(usize, Vec<Category>)> {
                panic!("base repo should not be used");
            }

            fn get_category_by_id(
                &self,
                _category_id: i32,
                _hub_id: i32,
            ) -> RepositoryResult<Option<Category>> {
                panic!("base repo should not be used");
            }
        }

        impl ProductReader for PanicRepo {
            fn get_product_by_id(
                &self,
                _id: i32,
                _hub_id: i32,
            ) -> RepositoryResult<Option<Product>> {
                panic!("base repo should not be used");
            }

            fn list_products(
                &self,
                _query: ProductListQuery,
            ) -> RepositoryResult<(usize, Vec<Product>)> {
                panic!("base repo should not be used");
            }
        }

        impl TagReader for PanicRepo {
            fn list_tags(&self, _query: TagListQuery) -> RepositoryResult<(usize, Vec<Tag>)> {
                panic!("base repo should not be used");
            }
        }

        impl PriceLevelReader for PanicRepo {
            fn get_price_level_by_id(
                &self,
                _id: i32,
                _hub_id: i32,
            ) -> RepositoryResult<Option<PriceLevel>> {
                panic!("base repo should not be used");
            }

            fn list_price_levels(
                &self,
                _query: PriceLevelListQuery,
            ) -> RepositoryResult<(usize, Vec<PriceLevel>)> {
                panic!("base repo should not be used");
            }
        }

        struct StaticRepo {
            categories: Vec<Category>,
            products: Vec<Product>,
            tags: Vec<Tag>,
            price_levels: Vec<PriceLevel>,
        }

        impl CategoryReader for StaticRepo {
            fn list_categories(
                &self,
                _query: CategoryTreeQuery,
            ) -> RepositoryResult<(usize, Vec<Category>)> {
                Ok((self.categories.len(), self.categories.clone()))
            }

            fn get_category_by_id(
                &self,
                category_id: i32,
                _hub_id: i32,
            ) -> RepositoryResult<Option<Category>> {
                Ok(self
                    .categories
                    .iter()
                    .find(|c| c.id == category_id)
                    .cloned())
            }
        }

        impl ProductReader for StaticRepo {
            fn get_product_by_id(
                &self,
                id: i32,
                _hub_id: i32,
            ) -> RepositoryResult<Option<Product>> {
                Ok(self.products.iter().find(|p| p.id == id).cloned())
            }

            fn list_products(
                &self,
                _query: ProductListQuery,
            ) -> RepositoryResult<(usize, Vec<Product>)> {
                Ok((self.products.len(), self.products.clone()))
            }
        }

        impl TagReader for StaticRepo {
            fn list_tags(&self, _query: TagListQuery) -> RepositoryResult<(usize, Vec<Tag>)> {
                Ok((self.tags.len(), self.tags.clone()))
            }
        }

        impl PriceLevelReader for StaticRepo {
            fn get_price_level_by_id(
                &self,
                id: i32,
                _hub_id: i32,
            ) -> RepositoryResult<Option<PriceLevel>> {
                Ok(self.price_levels.iter().find(|p| p.id == id).cloned())
            }

            fn list_price_levels(
                &self,
                _query: PriceLevelListQuery,
            ) -> RepositoryResult<(usize, Vec<PriceLevel>)> {
                Ok((self.price_levels.len(), self.price_levels.clone()))
            }
        }

        let static_repo = StaticRepo {
            categories: vec![Category {
                id: 1,
                hub_id: 1,
                parent_id: None,
                name: "Coffee".to_string(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            }],
            products: vec![Product {
                id: 1,
                hub_id: 1,
                name: "Coffee".to_string(),
                sku: None,
                description: None,
                units: None,
                currency: "USD".to_string(),
                is_archived: false,
                category_id: None,
                price_levels: Vec::new(),
                tags: Vec::new(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_urls: Vec::new(),
            }],
            tags: vec![Tag {
                id: 1,
                hub_id: 1,
                name: "Organic".to_string(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
            }],
            price_levels: vec![PriceLevel {
                id: 1,
                hub_id: 1,
                name: "Default".to_string(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: true,
            }],
        };

        let context: StoreClientHandle =
            StoreClientContext::new(Arc::new(static_repo) as Arc<dyn StoreClientRepository>);

        let categories = load_store_categories(
            &PanicRepo,
            1,
            StoreCategoryFilters::default(),
            Some(&context),
        )
        .expect("load categories from context");
        assert_eq!(categories.len(), 1);

        let products = load_store_products(
            &PanicRepo,
            1,
            StoreProductFilters::default(),
            Some(&context),
        )
        .expect("load products from context");
        assert_eq!(products.len(), 1);

        let tags = load_store_tags(&PanicRepo, 1, Some(&context)).expect("load tags from context");
        assert_eq!(tags.len(), 1);
    }
}
