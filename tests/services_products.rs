use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_orders::domain::{price_level::NewPriceLevel, product::ProductListQuery};
use pushkind_orders::forms::products::{AddProductForm, AddProductPriceLevelForm};
use pushkind_orders::repository::{DieselRepository, PriceLevelWriter, ProductReader};
use pushkind_orders::services::products;
use pushkind_orders::{SERVICE_ACCESS_ROLE, services::ServiceError};

mod common;

#[test]
fn create_product_stores_price_levels() {
    let test_db = common::TestDb::new("service_create_product_stores_price_levels.db");
    let repo = DieselRepository::new(test_db.pool());

    repo.create_price_level(&NewPriceLevel::new(1, "Retail"))
        .expect("create price level");

    let user = AuthenticatedUser {
        sub: "user".into(),
        email: "user@example.com".into(),
        hub_id: 1,
        name: "User".into(),
        roles: vec![SERVICE_ACCESS_ROLE.to_string()],
        exp: 0,
    };

    let form = AddProductForm {
        name: "Coffee".to_string(),
        sku: None,
        description: None,
        units: None,
        currency: "USD".to_string(),
        price_levels: vec![AddProductPriceLevelForm {
            price_level_id: 1,
            price: Some("12.50".to_string()),
        }],
    };

    let result = products::create_product(&repo, &user, form);
    assert!(
        result.is_ok(),
        "expected product creation to succeed: {result:?}"
    );

    let product = repo
        .list_products(ProductListQuery::new(1))
        .expect("list products")
        .1
        .pop()
        .expect("product should exist");

    assert_eq!(product.price_levels.len(), 1);
    assert_eq!(product.price_levels[0].price_level_id, 1);
    assert_eq!(product.price_levels[0].price_cents, 1250);
}

#[test]
fn create_product_requires_service_role() {
    let test_db = common::TestDb::new("service_create_product_requires_role.db");
    let repo = DieselRepository::new(test_db.pool());

    let user = AuthenticatedUser {
        sub: "user".into(),
        email: "user@example.com".into(),
        hub_id: 1,
        name: "User".into(),
        roles: vec![],
        exp: 0,
    };

    let form = AddProductForm {
        name: "Coffee".to_string(),
        sku: None,
        description: None,
        units: None,
        currency: "USD".to_string(),
        price_levels: Vec::new(),
    };

    let result = products::create_product(&repo, &user, form);
    assert!(matches!(result, Err(ServiceError::Unauthorized)));
}
