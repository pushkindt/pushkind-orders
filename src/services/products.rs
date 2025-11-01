use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::{Deserialize, Serialize};

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::{
    category::{Category, CategoryTreeQuery},
    price_level::{PriceLevel, PriceLevelListQuery},
    product::{Product, ProductListQuery},
    product_price_level::{NewProductPriceLevelRate, ProductPriceLevelRate},
    tag::{Tag, TagListQuery},
};
use crate::forms::products::{
    AddProductForm, EditProductForm, NewProductUpload, UploadProductsForm,
};
use crate::repository::{
    CategoryReader, PriceLevelReader, ProductReader, ProductWriter, TagReader,
};
use crate::services::{ServiceError, ServiceResult};

/// Query parameters accepted by the products index page.
#[derive(Debug, Default, Deserialize)]
pub struct ProductsQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page requested by the UI (1-based).
    pub page: Option<usize>,
    /// Whether archived items should be included in the response.
    #[serde(default)]
    pub show_archived: bool,
}

/// Data required to render the products index template.
pub struct ProductsPageData {
    /// Paginated list of products displayed in the table.
    pub products: Paginated<ProductView>,
    /// Search query echoed back to the view when present.
    pub search: Option<String>,
    /// All price levels used to render the modal form.
    pub price_levels: Vec<PriceLevel>,
    /// All available categories for the add product form.
    pub categories: Vec<Category>,
    /// All available tags for the edit product modal.
    pub tags: Vec<Tag>,
    /// Whether archived items were requested.
    pub show_archived: bool,
}

/// Loads the products overview page.
pub fn load_products_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: ProductsQuery,
) -> ServiceResult<ProductsPageData>
where
    R: ProductReader + PriceLevelReader + CategoryReader + TagReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let ProductsQuery {
        search,
        page,
        show_archived,
    } = query;

    let page = page.unwrap_or(1);
    let mut list_query = ProductListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(search_term) = search.as_ref() {
        list_query = list_query.search(search_term);
    }

    if show_archived {
        list_query = list_query.include_archived();
    }

    let (total, items) = repo.list_products(list_query).map_err(ServiceError::from)?;
    let (_, price_levels) = repo
        .list_price_levels(PriceLevelListQuery::new(user.hub_id))
        .map_err(ServiceError::from)?;

    let (_, mut categories) = repo
        .list_categories(CategoryTreeQuery::new(user.hub_id))
        .map_err(ServiceError::from)?;
    let category_lookup: HashMap<i32, String> = categories
        .iter()
        .map(|category| (category.id, category.name.clone()))
        .collect();
    categories.retain(|category| !category.is_archived);
    categories.sort_by(|a, b| a.name.cmp(&b.name));

    let (_, mut tags) = repo
        .list_tags(TagListQuery::new(user.hub_id))
        .map_err(ServiceError::from)?;
    tags.sort_by(|a, b| a.name.cmp(&b.name));

    let level_lookup: HashMap<i32, &PriceLevel> =
        price_levels.iter().map(|level| (level.id, level)).collect();

    let view_items: Vec<ProductView> = items
        .into_iter()
        .map(|product| ProductView::from_product(product, &level_lookup, &category_lookup))
        .collect();

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let products = Paginated::new(view_items, page, total_pages);

    Ok(ProductsPageData {
        products,
        search,
        price_levels,
        categories,
        tags,
        show_archived,
    })
}

/// Creates a new product for the authenticated user's hub.
pub fn create_product<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddProductForm,
) -> ServiceResult<Product>
where
    R: ProductWriter + PriceLevelReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let price_levels = fetch_all_price_levels(repo, user.hub_id)?;

    let payload = form
        .into_new_product_with_prices(user.hub_id, &price_levels)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    persist_new_product(repo, user.hub_id, payload)
}

/// Imports products from an uploaded CSV file.
pub fn import_products<R>(
    repo: &R,
    user: &AuthenticatedUser,
    mut form: UploadProductsForm,
) -> ServiceResult<usize>
where
    R: ProductWriter + PriceLevelReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let price_levels = fetch_all_price_levels(repo, user.hub_id)?;

    let uploads = form
        .into_new_products(user.hub_id, &price_levels)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let mut created = 0usize;
    for upload in uploads {
        persist_new_product(repo, user.hub_id, upload)?;
        created += 1;
    }

    Ok(created)
}

/// Updates an existing product for the authenticated user's hub.
pub fn update_product<R>(
    repo: &R,
    user: &AuthenticatedUser,
    product_id: i32,
    form: EditProductForm,
) -> ServiceResult<Product>
where
    R: ProductReader + ProductWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if product_id <= 0 {
        return Err(ServiceError::Form(
            "Некорректный идентификатор товара.".to_string(),
        ));
    }

    let payload = form
        .into_update_product()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let updates = payload.product;
    let tag_ids = payload.tag_ids;

    repo.replace_product_tags(product_id, user.hub_id, &tag_ids)
        .map_err(ServiceError::from)?;

    repo.update_product(product_id, user.hub_id, &updates)
        .map_err(ServiceError::from)
}

fn fetch_all_price_levels<R>(repo: &R, hub_id: i32) -> ServiceResult<Vec<PriceLevel>>
where
    R: PriceLevelReader + ?Sized,
{
    let query = PriceLevelListQuery::new(hub_id);
    let (_, price_levels) = repo.list_price_levels(query).map_err(ServiceError::from)?;
    Ok(price_levels)
}

fn persist_new_product<R>(
    repo: &R,
    hub_id: i32,
    payload: NewProductUpload,
) -> ServiceResult<Product>
where
    R: ProductWriter + ?Sized,
{
    let created = repo
        .create_product(&payload.product)
        .map_err(ServiceError::from)?;

    if payload.price_levels.is_empty() {
        return Ok(created);
    }

    let rates: Vec<NewProductPriceLevelRate> = payload
        .price_levels
        .iter()
        .map(|rate| {
            NewProductPriceLevelRate::new(created.id, rate.price_level_id, rate.price_cents)
        })
        .collect();

    if let Err(err) = repo.replace_product_price_levels(created.id, hub_id, &rates) {
        log::error!(
            "Failed to attach price levels to product {}: {err}",
            created.id
        );
        if let Err(delete_err) = repo.delete_product(created.id, hub_id) {
            log::error!(
                "Failed to roll back product {} after price level error: {delete_err}",
                created.id
            );
        }
        return Err(ServiceError::from(err));
    }

    Ok(created)
}

/// View model exposed to the products index template.
#[derive(Debug, Serialize)]
pub struct ProductView {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub units: Option<String>,
    pub currency: String,
    pub is_archived: bool,
    pub category_id: Option<i32>,
    pub category_name: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
    pub price_levels: Vec<ProductPriceLevelView>,
    pub tags: Vec<ProductTagView>,
}

impl ProductView {
    fn from_product(
        product: crate::domain::product::Product,
        level_lookup: &HashMap<i32, &PriceLevel>,
        category_lookup: &HashMap<i32, String>,
    ) -> Self {
        let crate::domain::product::Product {
            id,
            hub_id,
            name,
            sku,
            description,
            units,
            currency,
            is_archived,
            category_id,
            price_levels,
            tags,
            created_at: _,
            updated_at,
            ..
        } = product;

        let price_levels = price_levels
            .into_iter()
            .flat_map(|rate| ProductPriceLevelView::from_rate(rate, level_lookup))
            .collect();

        let tags = tags.into_iter().map(ProductTagView::from_tag).collect();

        Self {
            id,
            hub_id,
            name,
            sku,
            description,
            units,
            currency,
            is_archived,
            category_id,
            category_name: category_id.and_then(|id| category_lookup.get(&id).cloned()),
            updated_at,
            price_levels,
            tags,
        }
    }
}

/// View model for a product price level entry.
#[derive(Debug, Serialize)]
pub struct ProductPriceLevelView {
    pub price_level_id: i32,
    pub price_level_name: String,
    pub price_cents: i32,
    pub price_formatted: String,
}

/// View model for a product tag entry.
#[derive(Debug, Serialize)]
pub struct ProductTagView {
    pub id: i32,
    pub name: String,
}

impl ProductTagView {
    fn from_tag(tag: Tag) -> Self {
        Self {
            id: tag.id,
            name: tag.name,
        }
    }
}

impl ProductPriceLevelView {
    fn from_rate(
        rate: ProductPriceLevelRate,
        level_lookup: &HashMap<i32, &PriceLevel>,
    ) -> Option<Self> {
        let level = level_lookup.get(&rate.price_level_id)?;
        let price_formatted = format!("{:.2}", rate.price_cents as f64 / 100.0);

        Some(Self {
            price_level_id: rate.price_level_id,
            price_level_name: level.name.clone(),
            price_cents: rate.price_cents,
            price_formatted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::Value;
    use std::collections::VecDeque;
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::{Arc, Mutex};

    use crate::domain::{
        category::Category, price_level::PriceLevel, product::Product,
        product_price_level::ProductPriceLevelRate,
    };
    use crate::forms::products::{
        AddProductForm, AddProductPriceLevelForm, EditProductForm, UploadProductsForm,
    };
    use crate::repository::mock::{
        MockCategoryReader, MockPriceLevelReader, MockProductReader, MockProductWriter,
        MockTagReader,
    };
    use actix_multipart::form::tempfile::TempFile;
    use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};
    use tempfile::NamedTempFile;

    fn datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .unwrap_or_default()
    }

    fn sample_product(
        id: i32,
        hub_id: i32,
        name: &str,
        price_levels: Vec<ProductPriceLevelRate>,
    ) -> Product {
        Product {
            id,
            hub_id,
            name: name.to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "USD".to_string(),
            is_archived: false,
            category_id: None,
            price_levels,
            tags: Vec::new(),
            created_at: datetime(),
            updated_at: datetime(),
        }
    }

    fn user_with_role(role: &str) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 11,
            name: "User".to_string(),
            roles: vec![role.to_string()],
            exp: 0,
        }
    }

    #[test]
    fn load_products_page_requires_role() {
        let repo = FakeRepo::new();
        let user = AuthenticatedUser {
            sub: "user".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 11,
            name: "User".to_string(),
            roles: Vec::new(),
            exp: 0,
        };

        let result = load_products_page(&repo, &user, ProductsQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_products_page_returns_data() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let query = ProductsQuery {
            search: Some("coffee".to_string()),
            page: Some(3),
            show_archived: false,
        };

        let expected_hub = user.hub_id;

        let price_level_rows = vec![
            price_level(10, expected_hub, "Retail"),
            price_level(11, expected_hub, "Wholesale"),
        ];
        let category_rows = vec![
            category(31, expected_hub, "Accessories", false),
            category(32, expected_hub, "Archived", true),
            category(33, expected_hub, "Beverages", false),
        ];
        let tag_rows = vec![
            tag(41, expected_hub, "Seasonal"),
            tag(42, expected_hub, "Popular"),
        ];

        let product_tag_rows = tag_rows.clone();

        repo.product_reader
            .expect_list_products()
            .times(1)
            .withf(move |qry| {
                assert_eq!(qry.hub_id, expected_hub);
                assert_eq!(qry.search.as_deref(), Some("coffee"));
                match &qry.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 3);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be set"),
                }
                true
            })
            .returning(move |_| {
                let mut product_a = sample_product(
                    1,
                    expected_hub,
                    "Coffee A",
                    vec![ProductPriceLevelRate {
                        id: 1,
                        product_id: 1,
                        price_level_id: 10,
                        price_cents: 1299,
                        created_at: datetime(),
                        updated_at: datetime(),
                    }],
                );
                product_a.tags = vec![product_tag_rows[0].clone()];
                product_a.category_id = Some(31);

                let mut product_b = sample_product(
                    2,
                    expected_hub,
                    "Coffee B",
                    vec![ProductPriceLevelRate {
                        id: 2,
                        product_id: 2,
                        price_level_id: 11,
                        price_cents: 1599,
                        created_at: datetime(),
                        updated_at: datetime(),
                    }],
                );

                product_b.tags = vec![product_tag_rows[1].clone()];
                product_b.category_id = Some(33);

                Ok((27, vec![product_a, product_b]))
            });

        repo.price_level_reader
            .expect_list_price_levels()
            .times(1)
            .returning(move |_| Ok((price_level_rows.len(), price_level_rows.clone())));

        let categories_response = category_rows.clone();
        let categories_len = categories_response.len();
        repo.category_reader
            .expect_list_categories()
            .times(1)
            .withf(move |qry| {
                assert_eq!(qry.hub_id, expected_hub);
                assert!(!qry.include_archived);
                true
            })
            .returning(move |_| Ok((categories_len, categories_response.clone())));

        let tags_response = tag_rows.clone();
        let tags_len = tags_response.len();
        repo.tag_reader
            .expect_list_tags()
            .times(1)
            .withf(move |qry| {
                assert_eq!(qry.hub_id, expected_hub);
                assert!(qry.search.is_none());
                assert!(qry.pagination.is_none());
                true
            })
            .returning(move |_| Ok((tags_len, tags_response.clone())));

        let result = load_products_page(&repo, &user, query);

        let data = result.expect("expected success");
        assert_eq!(data.search.as_deref(), Some("coffee"));
        assert!(!data.show_archived);
        assert_eq!(data.price_levels.len(), 2);
        assert_eq!(data.categories.len(), 2);
        assert_eq!(data.tags.len(), 2);
        let category_names: Vec<&str> = data
            .categories
            .iter()
            .map(|category| category.name.as_str())
            .collect();
        assert_eq!(category_names, vec!["Accessories", "Beverages"]);

        let tag_names: Vec<&str> = data.tags.iter().map(|tag| tag.name.as_str()).collect();
        assert_eq!(tag_names, vec!["Popular", "Seasonal"]);

        let serialized = serde_json::to_value(&data.products).expect("serialization");
        assert_eq!(serialized.get("page").and_then(Value::as_u64), Some(3));

        let items = serialized
            .get("items")
            .and_then(Value::as_array)
            .expect("items array");
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].get("category_name").and_then(Value::as_str),
            Some("Accessories")
        );
        assert_eq!(
            items[1].get("category_name").and_then(Value::as_str),
            Some("Beverages")
        );

        let first_price_levels = items[0]
            .get("price_levels")
            .and_then(Value::as_array)
            .expect("price levels array");
        assert_eq!(first_price_levels.len(), 1);
        assert_eq!(
            first_price_levels[0]
                .get("price_level_name")
                .and_then(Value::as_str),
            Some("Retail")
        );
        assert_eq!(
            first_price_levels[0]
                .get("price_formatted")
                .and_then(Value::as_str),
            Some("12.99")
        );

        let first_tags = items[0]
            .get("tags")
            .and_then(Value::as_array)
            .expect("tags array");
        assert_eq!(first_tags.len(), 1);
        assert_eq!(
            first_tags[0].get("name").and_then(Value::as_str),
            Some("Seasonal")
        );
    }

    #[test]
    fn load_products_page_respects_show_archived_flag() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let expected_hub = user.hub_id;

        repo.product_reader
            .expect_list_products()
            .times(1)
            .withf(move |qry| {
                assert_eq!(qry.hub_id, expected_hub);
                assert!(qry.include_archived);
                true
            })
            .returning(move |_| Ok((0, Vec::new())));

        repo.price_level_reader
            .expect_list_price_levels()
            .times(1)
            .returning(move |_| Ok((0, Vec::new())));

        repo.category_reader
            .expect_list_categories()
            .times(1)
            .returning(move |_| Ok((0, Vec::new())));

        repo.tag_reader
            .expect_list_tags()
            .times(1)
            .returning(move |_| Ok((0, Vec::new())));

        let result = load_products_page(
            &repo,
            &user,
            ProductsQuery {
                search: None,
                page: None,
                show_archived: true,
            },
        );

        let data = result.expect("expected success");
        assert!(data.show_archived);
        assert!(data.categories.is_empty());
        assert!(data.tags.is_empty());
    }

    #[test]
    fn create_product_requires_role() {
        let repo = FakeRepo::new();
        let user = AuthenticatedUser {
            sub: "user".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 42,
            name: "User".to_string(),
            roles: Vec::new(),
            exp: 0,
        };

        let form = AddProductForm {
            name: "Widget".to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "USD".to_string(),
            category_id: None,
            price_levels: Vec::new(),
        };

        let result = create_product(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn create_product_persists_product_and_rates() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let hub_id = user.hub_id;
        let levels = vec![price_level(10, hub_id, "Retail")];

        repo.price_level_reader
            .expect_list_price_levels()
            .times(1)
            .returning(move |_| Ok((levels.len(), levels.clone())));

        repo.product_writer
            .expect_create_product()
            .times(1)
            .withf(move |new_product| {
                assert_eq!(new_product.hub_id, hub_id);
                assert_eq!(new_product.name, "Widget");
                assert_eq!(new_product.currency, "USD");
                assert_eq!(new_product.units.as_deref(), Some("Each"));
                true
            })
            .returning(move |_| Ok(sample_product(101, hub_id, "Widget", Vec::new())));

        let expected_hub = hub_id;
        repo.product_writer
            .expect_replace_product_price_levels()
            .times(1)
            .withf(move |product_id, scope_hub, rates| {
                assert_eq!(*product_id, 101);
                assert_eq!(*scope_hub, expected_hub);
                assert_eq!(rates.len(), 1);
                assert_eq!(rates[0].price_level_id, 10);
                assert_eq!(rates[0].price_cents, 1234);
                true
            })
            .returning(|_, _, _| Ok(()));

        let form = AddProductForm {
            name: " Widget ".to_string(),
            sku: Some(" SKU-1 ".to_string()),
            description: Some(" A great product ".to_string()),
            units: Some(" Each ".to_string()),
            currency: "usd".to_string(),
            category_id: None,
            price_levels: vec![AddProductPriceLevelForm {
                price_level_id: 10,
                price: Some("12.34".to_string()),
            }],
        };

        let result = create_product(&repo, &user, form).expect("expected success");
        assert_eq!(result.id, 101);
        assert_eq!(result.hub_id, hub_id);
        assert_eq!(result.name, "Widget");
    }

    #[test]
    fn create_product_rolls_back_when_rates_fail() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let hub_id = user.hub_id;
        let levels = vec![price_level(5, hub_id, "Retail")];

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(move |_| Ok((levels.len(), levels.clone())));

        repo.product_writer
            .expect_create_product()
            .returning(move |_| Ok(sample_product(7, hub_id, "Widget", Vec::new())));

        repo.product_writer
            .expect_replace_product_price_levels()
            .returning(|_, _, _| Err(RepositoryError::NotFound));

        let expected_hub_id = hub_id;
        repo.product_writer
            .expect_delete_product()
            .times(1)
            .withf(move |product_id, scope_hub| {
                assert_eq!(*product_id, 7);
                assert_eq!(*scope_hub, expected_hub_id);
                true
            })
            .returning(|_, _| Ok(()));

        let form = AddProductForm {
            name: "Widget".to_string(),
            sku: None,
            description: None,
            units: Some("Each".to_string()),
            currency: "USD".to_string(),
            category_id: None,
            price_levels: vec![AddProductPriceLevelForm {
                price_level_id: 5,
                price: Some("10.00".to_string()),
            }],
        };

        let result = create_product(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn import_products_creates_multiple_products() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let hub_id = user.hub_id;

        let levels = vec![
            price_level(1, hub_id, "Retail"),
            price_level(2, hub_id, "Wholesale"),
        ];

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(move |_| Ok((levels.len(), levels.clone())));

        let create_counter = Arc::new(Mutex::new(0));
        let create_counter_clone = create_counter.clone();

        repo.product_writer
            .expect_create_product()
            .times(2)
            .returning(move |new_product| {
                let mut counter = create_counter_clone.lock().unwrap();
                *counter += 1;
                let id = *counter;
                Ok(sample_product(
                    id,
                    new_product.hub_id,
                    new_product.name.as_str(),
                    Vec::new(),
                ))
            });

        let rate_counter = Arc::new(Mutex::new(0));
        let rate_counter_clone = rate_counter.clone();

        repo.product_writer
            .expect_replace_product_price_levels()
            .times(2)
            .returning(move |product_id, scope_hub, rates| {
                let mut idx = rate_counter_clone.lock().unwrap();
                match *idx {
                    0 => {
                        assert_eq!(product_id, 1);
                        assert_eq!(scope_hub, hub_id);
                        assert_eq!(rates.len(), 2);
                        assert_eq!(rates[0].price_level_id, 1);
                        assert_eq!(rates[0].price_cents, 1234);
                        assert_eq!(rates[1].price_level_id, 2);
                        assert_eq!(rates[1].price_cents, 990);
                    }
                    1 => {
                        assert_eq!(product_id, 2);
                        assert_eq!(scope_hub, hub_id);
                        assert_eq!(rates.len(), 1);
                        assert_eq!(rates[0].price_level_id, 1);
                        assert_eq!(rates[0].price_cents, 750);
                    }
                    _ => panic!("unexpected additional rate call"),
                }
                *idx += 1;
                Ok(())
            });

        let csv = "\
name,currency,Retail,Wholesale
Apple,USD,12.34,9.90
Banana,USD,7.50,
";
        let form = build_upload_form(csv);

        let result = import_products(&repo, &user, form).expect("expected success");

        assert_eq!(result, 2);
    }

    #[test]
    fn import_products_requires_role() {
        let repo = FakeRepo::new();
        let user = AuthenticatedUser {
            sub: "user".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 42,
            name: "User".to_string(),
            roles: Vec::new(),
            exp: 0,
        };

        let form = build_upload_form("name,currency\nWidget,USD\n");

        let result = import_products(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_product_requires_role() {
        let repo = FakeRepo::new();
        let user = AuthenticatedUser {
            sub: "user".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 11,
            name: "User".to_string(),
            roles: Vec::new(),
            exp: 0,
        };

        let form = EditProductForm {
            name: None,
            sku: None,
            description: None,
            units: None,
            currency: None,
            is_archived: None,
            category_id: None,
            tag_ids: Vec::new(),
        };

        let result = update_product(&repo, &user, 1, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_product_returns_not_found_for_missing_product() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let product_id = 24;
        let hub_id = user.hub_id;

        repo.product_reader
            .expect_get_product_by_id()
            .times(1)
            .withf(move |id, hub| *id == product_id && *hub == hub_id)
            .returning(|_, _| Ok(None));

        let form = EditProductForm {
            name: Some("Updated".to_string()),
            sku: None,
            description: None,
            units: None,
            currency: Some("usd".to_string()),
            is_archived: Some(false),
            category_id: None,
            tag_ids: vec!["3".to_string(), "5".to_string()],
        };

        let result = update_product(&repo, &user, product_id, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn update_product_applies_changes() {
        let mut repo = FakeRepo::new();
        let user = user_with_role(SERVICE_ACCESS_ROLE);
        let product_id = 7;
        let hub_id = user.hub_id;

        let mut base_product = sample_product(product_id, hub_id, "Espresso", Vec::new());
        base_product.sku = Some("ESP-1".to_string());
        base_product.description = Some("Strong".to_string());
        base_product.units = Some("kg".to_string());
        base_product.is_archived = false;
        base_product.category_id = Some(5);
        base_product.tags = vec![tag(40, hub_id, "Legacy")];

        let previous_updated_at = base_product.updated_at;
        let reader_product = base_product.clone();
        let writer_product = base_product.clone();
        let final_product = {
            let mut product = writer_product.clone();
            product.name = "Espresso Deluxe".to_string();
            product.currency = "EUR".to_string();
            product.sku = None;
            product.description = None;
            product.units = Some("pack".to_string());
            product.is_archived = true;
            product.category_id = None;
            product.tags = vec![tag(42, hub_id, "Featured"), tag(99, hub_id, "Top Seller")];
            product
        };

        let reader_queue = Arc::new(Mutex::new(VecDeque::from(vec![
            Some(reader_product.clone()),
            Some(final_product.clone()),
        ])));
        let reader_queue_clone = reader_queue.clone();

        repo.product_reader
            .expect_get_product_by_id()
            .times(2)
            .withf(move |id, hub| *id == product_id && *hub == hub_id)
            .returning(move |_, _| {
                Ok(reader_queue_clone
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(None))
            });

        repo.product_writer
            .expect_update_product()
            .times(1)
            .withf(move |id, hub, updates| {
                assert_eq!((*id, *hub), (product_id, hub_id));
                assert_eq!(updates.name.as_str(), "Espresso Deluxe");
                assert_eq!(updates.currency.as_str(), "EUR");
                assert!(updates.sku.is_none());
                assert!(updates.description.is_none());
                assert_eq!(updates.units.as_deref(), Some("pack"));
                assert!(updates.is_archived);
                assert!(updates.category_id.is_none());
                assert!(updates.updated_at > previous_updated_at);
                true
            })
            .returning(move |_, _, updates| {
                let mut updated = writer_product.clone();
                updated.name = updates.name.clone();
                updated.currency = updates.currency.clone();
                updated.sku = updates.sku.clone();
                updated.description = updates.description.clone();
                updated.units = updates.units.clone();
                updated.is_archived = updates.is_archived;
                updated.category_id = updates.category_id;
                updated.updated_at = updates.updated_at;
                Ok(updated)
            });

        repo.product_writer
            .expect_replace_product_tags()
            .times(1)
            .withf(move |id, hub, tags| {
                assert_eq!((*id, *hub), (product_id, hub_id));
                assert_eq!(tags, &[42, 99]);
                true
            })
            .returning(|_, _, _| Ok(()));

        let form = EditProductForm {
            name: Some("  Espresso Deluxe  ".to_string()),
            sku: Some("   ".to_string()),         // clears SKU
            description: Some("   ".to_string()), // clears description
            units: Some("  pack ".to_string()),
            currency: Some(" eur ".to_string()),
            is_archived: Some(true),
            category_id: Some("0".to_string()), // clears category
            tag_ids: vec!["42".to_string(), "99".to_string()],
        };

        let result =
            update_product(&repo, &user, product_id, form).expect("expected update to succeed");

        assert_eq!(result.name, "Espresso Deluxe");
        assert_eq!(result.currency, "EUR");
        assert!(result.sku.is_none());
        assert!(result.description.is_none());
        assert_eq!(result.units.as_deref(), Some("pack"));
        assert!(result.is_archived);
        assert!(result.category_id.is_none());
        assert_eq!(result.tags, final_product.tags);
    }

    struct FakeRepo {
        product_reader: MockProductReader,
        product_writer: MockProductWriter,
        price_level_reader: MockPriceLevelReader,
        category_reader: MockCategoryReader,
        tag_reader: MockTagReader,
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                product_reader: MockProductReader::new(),
                product_writer: MockProductWriter::new(),
                price_level_reader: MockPriceLevelReader::new(),
                category_reader: MockCategoryReader::new(),
                tag_reader: MockTagReader::new(),
            }
        }
    }

    impl ProductReader for FakeRepo {
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

    impl PriceLevelReader for FakeRepo {
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

    impl CategoryReader for FakeRepo {
        fn list_categories(
            &self,
            query: CategoryTreeQuery,
        ) -> RepositoryResult<(usize, Vec<Category>)> {
            self.category_reader.list_categories(query)
        }

        fn get_category_by_id(
            &self,
            category_id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Option<Category>> {
            self.category_reader.get_category_by_id(category_id, hub_id)
        }
    }

    impl TagReader for FakeRepo {
        fn list_tags(&self, query: TagListQuery) -> RepositoryResult<(usize, Vec<Tag>)> {
            self.tag_reader.list_tags(query)
        }
    }

    impl ProductWriter for FakeRepo {
        fn create_product(
            &self,
            new_product: &crate::domain::product::NewProduct,
        ) -> RepositoryResult<Product> {
            self.product_writer.create_product(new_product)
        }

        fn update_product(
            &self,
            product_id: i32,
            hub_id: i32,
            updates: &crate::domain::product::UpdateProduct,
        ) -> RepositoryResult<Product> {
            self.product_writer
                .update_product(product_id, hub_id, updates)
        }

        fn delete_product(&self, product_id: i32, hub_id: i32) -> RepositoryResult<()> {
            self.product_writer.delete_product(product_id, hub_id)
        }

        fn replace_product_price_levels(
            &self,
            product_id: i32,
            hub_id: i32,
            rates: &[NewProductPriceLevelRate],
        ) -> RepositoryResult<()> {
            self.product_writer
                .replace_product_price_levels(product_id, hub_id, rates)
        }

        fn replace_product_tags(
            &self,
            product_id: i32,
            hub_id: i32,
            tag_ids: &[i32],
        ) -> RepositoryResult<()> {
            self.product_writer
                .replace_product_tags(product_id, hub_id, tag_ids)
        }
    }

    fn build_upload_form(csv: &str) -> UploadProductsForm {
        let mut file = NamedTempFile::new().expect("create temp file");
        file.write_all(csv.as_bytes()).expect("write csv contents");
        file.as_file_mut()
            .seek(SeekFrom::Start(0))
            .expect("rewind csv");

        UploadProductsForm {
            csv: TempFile {
                file,
                content_type: None,
                file_name: Some("products.csv".to_string()),
                size: csv.len(),
            },
        }
    }

    fn price_level(id: i32, hub_id: i32, name: &str) -> PriceLevel {
        PriceLevel {
            id,
            hub_id,
            name: name.to_string(),
            created_at: datetime(),
            updated_at: datetime(),
            is_default: false,
        }
    }

    fn category(id: i32, hub_id: i32, name: &str, is_archived: bool) -> Category {
        Category {
            id,
            hub_id,
            parent_id: None,
            name: name.to_string(),
            description: None,
            is_archived,
            created_at: datetime(),
            updated_at: datetime(),
        }
    }

    fn tag(id: i32, hub_id: i32, name: &str) -> Tag {
        Tag {
            id,
            hub_id,
            name: name.to_string(),
            created_at: datetime(),
            updated_at: datetime(),
        }
    }
}
