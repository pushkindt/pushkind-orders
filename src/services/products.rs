use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::{Deserialize, Serialize};

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::{
    price_level::{PriceLevel, PriceLevelListQuery},
    product::{Product, ProductListQuery},
    product_price_level::{NewProductPriceLevelRate, ProductPriceLevelRate},
};
use crate::forms::products::{AddProductForm, NewProductUpload, UploadProductsForm};
use crate::repository::{PriceLevelReader, ProductReader, ProductWriter};
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
    R: ProductReader + PriceLevelReader + ?Sized,
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

    let level_lookup: HashMap<i32, &PriceLevel> =
        price_levels.iter().map(|level| (level.id, level)).collect();

    let view_items: Vec<ProductView> = items
        .into_iter()
        .map(|product| ProductView::from_product(product, &level_lookup))
        .collect();

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let products = Paginated::new(view_items, page, total_pages);

    Ok(ProductsPageData {
        products,
        search,
        price_levels,
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
    pub updated_at: chrono::NaiveDateTime,
    pub price_levels: Vec<ProductPriceLevelView>,
}

impl ProductView {
    fn from_product(
        product: crate::domain::product::Product,
        level_lookup: &HashMap<i32, &PriceLevel>,
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
            updated_at,
            ..
        } = product;

        let price_levels = price_levels
            .into_iter()
            .flat_map(|rate| ProductPriceLevelView::from_rate(rate, level_lookup))
            .collect();

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
            updated_at,
            price_levels,
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
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::{Arc, Mutex};

    use crate::domain::{
        price_level::PriceLevel, product::Product, product_price_level::ProductPriceLevelRate,
    };
    use crate::forms::products::{AddProductForm, AddProductPriceLevelForm, UploadProductsForm};
    use crate::repository::mock::{MockPriceLevelReader, MockProductReader, MockProductWriter};
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
                let product_a = sample_product(
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
                let product_b = sample_product(
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

                Ok((27, vec![product_a, product_b]))
            });

        repo.price_level_reader
            .expect_list_price_levels()
            .times(1)
            .returning(move |_| Ok((price_level_rows.len(), price_level_rows.clone())));

        let result = load_products_page(&repo, &user, query);

        let data = result.expect("expected success");
        assert_eq!(data.search.as_deref(), Some("coffee"));
        assert!(!data.show_archived);
        assert_eq!(data.price_levels.len(), 2);

        let serialized = serde_json::to_value(&data.products).expect("serialization");
        assert_eq!(serialized.get("page").and_then(Value::as_u64), Some(3));

        let items = serialized
            .get("items")
            .and_then(Value::as_array)
            .expect("items array");
        assert_eq!(items.len(), 2);

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

    struct FakeRepo {
        product_reader: MockProductReader,
        product_writer: MockProductWriter,
        price_level_reader: MockPriceLevelReader,
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                product_reader: MockProductReader::new(),
                product_writer: MockProductWriter::new(),
                price_level_reader: MockPriceLevelReader::new(),
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
        }
    }
}
