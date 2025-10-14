use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::{Deserialize, Serialize};

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::{
    price_level::{PriceLevel, PriceLevelListQuery},
    product::ProductListQuery,
    product_price_level::ProductPriceLevelRate,
};
use crate::repository::{PriceLevelReader, ProductReader};
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

/// View model exposed to the products index template.
#[derive(Debug, Serialize)]
pub struct ProductView {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub currency: String,
    pub is_archived: bool,
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
            currency,
            is_archived,
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
            currency,
            is_archived,
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

    use crate::domain::{
        price_level::PriceLevel, product::Product, product_price_level::ProductPriceLevelRate,
    };
    use crate::repository::mock::{MockPriceLevelReader, MockProductReader};
    use pushkind_common::repository::errors::RepositoryResult;

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
            currency: "USD".to_string(),
            is_archived: false,
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

    struct FakeRepo {
        product_reader: MockProductReader,
        price_level_reader: MockPriceLevelReader,
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                product_reader: MockProductReader::new(),
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
