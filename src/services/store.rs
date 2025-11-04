use std::sync::Arc;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::{
    category::{Category, CategoryTreeQuery},
    product::Product,
    tag::{Tag, TagListQuery},
};
use crate::repository::{CategoryReader, ProductReader, TagReader};
use crate::services::{ServiceError, ServiceResult};

/// Trait implemented by repositories that expose read access required by the
/// storefront service layer.
pub trait StoreClientRepository: CategoryReader + ProductReader + TagReader + Send + Sync {}

impl<T> StoreClientRepository for T where T: CategoryReader + ProductReader + TagReader + Send + Sync
{}

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
pub struct StoreCategory {
    /// Identifier of the category.
    pub id: i32,
    /// Parent identifier when the category belongs to a tree.
    pub parent_id: Option<i32>,
    /// Name displayed to users.
    pub name: String,
    /// Optional descriptive text.
    pub description: Option<String>,
}

impl From<Category> for StoreCategory {
    fn from(value: Category) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent_id,
            name: value.name,
            description: value.description,
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

/// Product payload formatted for storefront consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Tags attached to the product.
    pub tags: Vec<StoreTag>,
    /// Timestamp representing when the product was last updated.
    pub updated_at: NaiveDateTime,
}

impl From<Product> for StoreProduct {
    fn from(value: Product) -> Self {
        Self {
            id: value.id,
            category_id: value.category_id,
            name: value.name,
            sku: value.sku,
            description: value.description,
            units: value.units,
            currency: value.currency,
            tags: value.tags.into_iter().map(StoreTag::from).collect(),
            updated_at: value.updated_at,
        }
    }
}

/// Load categories available to a storefront for the provided hub.
pub fn load_store_categories<R>(
    repo: &R,
    hub_id: i32,
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

    let filtered = categories
        .into_iter()
        .filter(|category| !category.is_archived)
        .map(StoreCategory::from)
        .collect();

    Ok(filtered)
}

/// Load products available to a storefront for the provided hub.
pub fn load_store_products<R>(
    repo: &R,
    hub_id: i32,
    store_client: Option<&StoreClientHandle>,
) -> ServiceResult<Vec<StoreProduct>>
where
    R: ProductReader + ?Sized,
{
    let products = match store_client {
        Some(client) => {
            client
                .repository()
                .list_products(crate::domain::product::ProductListQuery::new(hub_id))
                .map_err(ServiceError::from)?
                .1
        }
        None => {
            repo.list_products(crate::domain::product::ProductListQuery::new(hub_id))
                .map_err(ServiceError::from)?
                .1
        }
    };

    let filtered = products
        .into_iter()
        .filter(|product| !product.is_archived)
        .map(StoreProduct::from)
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
    use crate::domain::product::ProductListQuery;
    use crate::repository::mock::{MockCategoryReader, MockProductReader};
    use pushkind_common::repository::errors::RepositoryResult;

    fn sample_timestamp() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
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
            },
        ];

        repo.expect_list_categories()
            .withf(|query| query.hub_id == 1 && !query.include_archived)
            .return_once(move |_| Ok((2, categories.clone())));

        let result = load_store_categories(&repo, 1, None).expect("load categories");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "Coffee");
    }

    #[test]
    fn load_products_includes_tags() {
        let mut repo = MockProductReader::new();
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
                price_levels: Vec::new(),
                tags: vec![Tag {
                    id: 1,
                    hub_id: 1,
                    name: "Organic".to_string(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                }],
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
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
            },
        ];

        repo.expect_list_products()
            .withf(|query| query.hub_id == 1 && !query.include_archived)
            .return_once(move |_| Ok((2, products.clone())));

        let result = load_store_products(&repo, 1, None).expect("load products");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "Coffee");
        assert_eq!(result[0].tags.len(), 1);
        assert_eq!(result[0].tags[0].name, "Organic");
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

        struct StaticRepo {
            categories: Vec<Category>,
            products: Vec<Product>,
            tags: Vec<Tag>,
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
            }],
            tags: vec![Tag {
                id: 1,
                hub_id: 1,
                name: "Organic".to_string(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
            }],
        };

        let context: StoreClientHandle =
            StoreClientContext::new(Arc::new(static_repo) as Arc<dyn StoreClientRepository>);

        let categories = load_store_categories(&PanicRepo, 1, Some(&context))
            .expect("load categories from context");
        assert_eq!(categories.len(), 1);

        let products =
            load_store_products(&PanicRepo, 1, Some(&context)).expect("load products from context");
        assert_eq!(products.len(), 1);

        let tags = load_store_tags(&PanicRepo, 1, Some(&context)).expect("load tags from context");
        assert_eq!(tags.len(), 1);
    }
}
