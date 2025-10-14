use pushkind_common::repository::errors::RepositoryError;
use pushkind_orders::domain::{
    order::{NewOrder, OrderListQuery, OrderProduct, OrderStatus, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, ProductListQuery, UpdateProduct},
    product_price_level::NewProductPriceLevelRate,
    user::{NewUser, UpdateUser},
};
use pushkind_orders::models::product_price_level::NewProductPriceLevel as DbNewProductPriceLevel;
use pushkind_orders::repository::DieselRepository;
use pushkind_orders::repository::{
    OrderReader, OrderWriter, PriceLevelReader, PriceLevelWriter, ProductReader, ProductWriter,
    UserListQuery, UserReader, UserWriter,
};

mod common;

#[test]
fn test_user_repository_crud() {
    let test_db = common::TestDb::new("test_user_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());

    let alice_new = NewUser::new(1, "Alice".to_string(), "ALICE@example.com".to_string());
    let bob_new = NewUser::new(1, "Bob".to_string(), "bob@example.com".to_string());

    let alice = repo
        .create_user(&alice_new)
        .expect("failed to create Alice");
    let bob = repo.create_user(&bob_new).expect("failed to create Bob");

    assert_eq!(alice.name, "Alice");
    assert_eq!(alice.email, "alice@example.com");

    let fetched = repo
        .get_user_by_id(alice.id, 1)
        .expect("failed to fetch user")
        .expect("expected Alice to exist");
    assert_eq!(fetched.id, alice.id);

    assert!(
        repo.get_user_by_id(alice.id, 2)
            .expect("failed to fetch scoped user")
            .is_none()
    );

    let fetched_by_email = repo
        .get_user_by_email("alice@example.com", 1)
        .expect("failed to fetch by email")
        .expect("expected Alice via email");
    assert_eq!(fetched_by_email.id, alice.id);

    assert!(
        repo.get_user_by_email("alice@example.com", 2)
            .expect("failed to fetch by email scoped")
            .is_none()
    );

    let (total_all, users_all) = repo
        .list_users(UserListQuery::new(1))
        .expect("failed to list users");
    assert_eq!(total_all, 2);
    assert_eq!(users_all.len(), 2);

    let (total_filtered, users_filtered) = repo
        .list_users(UserListQuery::new(1).search("bob"))
        .expect("failed to search users");
    assert_eq!(total_filtered, 1);
    assert_eq!(users_filtered[0].id, bob.id);

    let updates = UpdateUser {
        name: "Alicia".to_string(),
    };

    let updated = repo
        .update_user(alice.id, 1, &updates)
        .expect("failed to update user");
    assert_eq!(updated.name, "Alicia");

    let err = repo
        .update_user(alice.id, 2, &updates)
        .expect_err("expected cross-hub update to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .delete_user(alice.id, 2)
        .expect_err("expected cross-hub delete to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_user(alice.id, 1)
        .expect("failed to delete user");
    assert!(
        repo.get_user_by_id(alice.id, 1)
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_after, users_after) = repo
        .list_users(UserListQuery::new(1))
        .expect("failed to list after delete");
    assert_eq!(total_after, 1);
    assert_eq!(users_after[0].id, bob.id);
}

#[test]
fn test_product_repository_crud() {
    let test_db = common::TestDb::new("test_product_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());

    let apple_new = NewProduct::new(1, "Apple", "USD")
        .with_sku("APL-1")
        .with_description("Fresh apple");
    let banana_new = NewProduct::new(1, "Banana", "USD");

    let apple = repo
        .create_product(&apple_new)
        .expect("failed to create apple product");
    let banana = repo
        .create_product(&banana_new)
        .expect("failed to create banana product");

    assert_eq!(apple.name, "Apple");
    assert_eq!(apple.sku.as_deref(), Some("APL-1"));
    assert!(apple.price_levels.is_empty());
    assert!(banana.price_levels.is_empty());

    let fetched = repo
        .get_product_by_id(apple.id, 1)
        .expect("failed to fetch by id")
        .expect("expected apple product");
    assert_eq!(fetched.id, apple.id);
    assert!(fetched.price_levels.is_empty());

    assert!(
        repo.get_product_by_id(apple.id, 2)
            .expect("failed to fetch cross-hub")
            .is_none()
    );

    let (total_all, products_all) = repo
        .list_products(ProductListQuery::new(1))
        .expect("failed to list products");
    assert_eq!(total_all, 2);
    assert_eq!(products_all.len(), 2);
    assert!(
        products_all
            .iter()
            .all(|product| product.price_levels.is_empty())
    );

    let (total_search, products_search) = repo
        .list_products(ProductListQuery::new(1).search("apple"))
        .expect("failed to search products");
    assert_eq!(total_search, 1);
    assert_eq!(products_search[0].id, apple.id);
    assert!(products_search[0].price_levels.is_empty());

    let (total_sku, products_sku) = repo
        .list_products(ProductListQuery::new(1).sku("APL-1"))
        .expect("failed to list by sku");
    assert_eq!(total_sku, 1);
    assert_eq!(products_sku[0].id, apple.id);

    let updates = UpdateProduct::new().archived(true).name("Apple Premium");

    let updated = repo
        .update_product(apple.id, 1, &updates)
        .expect("failed to update product");
    assert!(updated.is_archived);
    assert_eq!(updated.name, "Apple Premium");
    assert!(updated.price_levels.is_empty());

    let err = repo
        .update_product(apple.id, 2, &UpdateProduct::new().name("Intruder"))
        .expect_err("expected cross-hub update failure");
    assert!(matches!(err, RepositoryError::NotFound));

    let (total_visible, products_visible) = repo
        .list_products(ProductListQuery::new(1))
        .expect("failed to list non-archived");
    assert_eq!(total_visible, 1);
    assert_eq!(products_visible[0].id, banana.id);

    let (total_with_archived, products_with_archived) = repo
        .list_products(ProductListQuery::new(1).include_archived())
        .expect("failed to list including archived");
    assert_eq!(total_with_archived, 2);
    assert_eq!(products_with_archived.len(), 2);

    let err = repo
        .delete_product(apple.id, 2)
        .expect_err("expected cross-hub delete failure");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_product(apple.id, 1)
        .expect("failed to delete product");
    assert!(
        repo.get_product_by_id(apple.id, 1)
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_final, products_final) = repo
        .list_products(ProductListQuery::new(1).include_archived())
        .expect("failed final list");
    assert_eq!(total_final, 1);
    assert_eq!(products_final[0].id, banana.id);
}

#[test]
fn test_replace_product_price_levels() {
    let test_db = common::TestDb::new("test_replace_product_price_levels.db");
    let repo = DieselRepository::new(test_db.pool());

    let retail_level = repo
        .create_price_level(&NewPriceLevel::new(1, "Retail"))
        .expect("failed to create price level");
    let wholesale_level = repo
        .create_price_level(&NewPriceLevel::new(1, "Wholesale"))
        .expect("failed to create price level");

    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD"))
        .expect("failed to create product");

    let rates = vec![
        NewProductPriceLevelRate::new(product.id, retail_level.id, 1250),
        NewProductPriceLevelRate::new(product.id, wholesale_level.id, 990),
    ];

    repo.replace_product_price_levels(product.id, 1, &rates)
        .expect("failed to replace product price levels");

    let mut fetched = repo
        .get_product_by_id(product.id, 1)
        .expect("failed to fetch product")
        .expect("product should exist");

    fetched.price_levels.sort_by_key(|rate| rate.price_level_id);

    assert_eq!(fetched.price_levels.len(), 2);
    assert_eq!(fetched.price_levels[0].price_level_id, retail_level.id);
    assert_eq!(fetched.price_levels[0].price_cents, 1250);
    assert_eq!(fetched.price_levels[1].price_level_id, wholesale_level.id);
    assert_eq!(fetched.price_levels[1].price_cents, 990);

    let err = repo
        .replace_product_price_levels(product.id, 2, &rates)
        .expect_err("expected cross-hub update to fail");
    assert!(matches!(err, RepositoryError::NotFound));
}

#[test]
fn test_price_level_repository_crud() {
    let test_db = common::TestDb::new("test_price_level_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());

    let bronze_new = NewPriceLevel::new(1, " Bronze ");
    let silver_new = NewPriceLevel::new(1, "Silver");

    let bronze = repo
        .create_price_level(&bronze_new)
        .expect("failed to create bronze level");
    let silver = repo
        .create_price_level(&silver_new)
        .expect("failed to create silver level");

    assert_eq!(bronze.name, "Bronze");
    assert_eq!(silver.name, "Silver");

    let fetched = repo
        .get_price_level_by_id(bronze.id, 1)
        .expect("failed to fetch by id")
        .expect("expected bronze price level");
    assert_eq!(fetched.id, bronze.id);
    assert_eq!(fetched.name, "Bronze");

    assert!(
        repo.get_price_level_by_id(bronze.id, 2)
            .expect("failed to fetch cross-hub")
            .is_none()
    );

    let (total_all, levels_all) = repo
        .list_price_levels(PriceLevelListQuery::new(1))
        .expect("failed to list price levels");
    assert_eq!(total_all, 2);
    assert_eq!(levels_all.len(), 2);

    let (total_search, levels_search) = repo
        .list_price_levels(PriceLevelListQuery::new(1).search("Sil"))
        .expect("failed to search price levels");
    assert_eq!(total_search, 1);
    assert_eq!(levels_search[0].id, silver.id);

    let updates = UpdatePriceLevel::new().name("Gold");

    let updated = repo
        .update_price_level(bronze.id, 1, &updates)
        .expect("failed to update price level");
    assert_eq!(updated.name, "Gold");

    let err = repo
        .update_price_level(bronze.id, 2, &UpdatePriceLevel::new().name("Intruder"))
        .expect_err("expected cross-hub update failure");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .delete_price_level(bronze.id, 2)
        .expect_err("expected cross-hub delete failure");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_price_level(bronze.id, 1)
        .expect("failed to delete price level");
    assert!(
        repo.get_price_level_by_id(bronze.id, 1)
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_final, levels_final) = repo
        .list_price_levels(PriceLevelListQuery::new(1))
        .expect("failed to list after delete");
    assert_eq!(total_final, 1);
    assert_eq!(levels_final[0].id, silver.id);
}

#[test]
fn deleting_price_level_removes_product_rates() {
    use diesel::prelude::*;
    use pushkind_orders::schema::product_price_levels::dsl as product_rates;

    let test_db = common::TestDb::new("test_price_level_delete_cascade.db");
    let repo = DieselRepository::new(test_db.pool());

    let product = repo
        .create_product(&NewProduct::new(1, "Cascade Product", "USD"))
        .expect("failed to create product");
    let price_level = repo
        .create_price_level(&NewPriceLevel::new(1, "Cascade Level"))
        .expect("failed to create price level");

    {
        let mut conn = test_db
            .pool()
            .get()
            .expect("failed to acquire connection for insert");

        let new_rate = DbNewProductPriceLevel {
            product_id: product.id,
            price_level_id: price_level.id,
            price_cents: 1950,
        };

        diesel::insert_into(product_rates::product_price_levels)
            .values(&new_rate)
            .execute(&mut conn)
            .expect("failed to insert product price level");

        let existing: i64 = product_rates::product_price_levels
            .filter(product_rates::price_level_id.eq(price_level.id))
            .count()
            .get_result(&mut conn)
            .expect("failed to count inserted rates");
        assert_eq!(existing, 1);
    }

    repo.delete_price_level(price_level.id, 1)
        .expect("failed to delete price level");

    {
        let mut conn = test_db
            .pool()
            .get()
            .expect("failed to acquire connection for verification");
        let remaining: i64 = product_rates::product_price_levels
            .filter(product_rates::product_id.eq(product.id))
            .count()
            .get_result(&mut conn)
            .expect("failed to count remaining rates");
        assert_eq!(remaining, 0, "expected cascade delete to remove rates");
    }

    let updated_product = repo
        .get_product_by_id(product.id, 1)
        .expect("failed to fetch product after cascade")
        .expect("product should still exist");
    assert!(
        updated_product.price_levels.is_empty(),
        "product should have no price levels after cascade delete"
    );
}

#[test]
fn test_order_repository_crud() {
    let test_db = common::TestDb::new("test_order_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());

    let product_snapshot = OrderProduct::new("Apple", 150, "USD", 2)
        .with_sku("APL-1")
        .with_description("Fresh apple");

    let new_order = NewOrder::new(1, 300, "USD")
        .with_customer_id(42)
        .with_reference("REF-001")
        .with_notes("Handle with care")
        .with_status(OrderStatus::Pending)
        .with_products(vec![product_snapshot.clone()]);

    let order = repo
        .create_order(&new_order)
        .expect("failed to create order");
    assert_eq!(order.hub_id, 1);
    assert_eq!(order.status, OrderStatus::Pending);
    assert_eq!(order.products.len(), 1);
    assert_eq!(order.products[0].name, "Apple");

    let fetched = repo
        .get_order_by_id(order.id, 1)
        .expect("failed to fetch order")
        .expect("order should exist");
    assert_eq!(fetched.id, order.id);
    assert_eq!(fetched.products.len(), 1);

    assert!(
        repo.get_order_by_id(order.id, 2)
            .expect("failed scoped fetch")
            .is_none()
    );

    let (total_all, orders_all) = repo
        .list_orders(OrderListQuery::new(1))
        .expect("failed to list orders");
    assert_eq!(total_all, 1);
    assert_eq!(orders_all.len(), 1);

    let (total_status, orders_status) = repo
        .list_orders(OrderListQuery::new(1).status(OrderStatus::Pending))
        .expect("failed to filter by status");
    assert_eq!(total_status, 1);
    assert_eq!(orders_status[0].id, order.id);

    let (total_customer, orders_customer) = repo
        .list_orders(OrderListQuery::new(1).customer_id(42))
        .expect("failed to filter by customer");
    assert_eq!(total_customer, 1);
    assert_eq!(orders_customer[0].id, order.id);

    let (total_search, orders_search) = repo
        .list_orders(OrderListQuery::new(1).search("REF-001"))
        .expect("failed to search orders");
    assert_eq!(total_search, 1);
    assert_eq!(orders_search[0].id, order.id);

    let (total_none, _) = repo
        .list_orders(OrderListQuery::new(1).search("missing"))
        .expect("failed to search missing");
    assert_eq!(total_none, 0);

    let product_updates = vec![product_snapshot.clone().with_description("Sliced apple")];
    let updates = UpdateOrder::new()
        .status(OrderStatus::Processing)
        .notes(Some("Pack immediately"))
        .customer_id(Some(43))
        .products(product_updates.clone());

    let updated = repo
        .update_order(order.id, 1, &updates)
        .expect("failed to update order");
    assert_eq!(updated.status, OrderStatus::Processing);
    assert_eq!(updated.customer_id, Some(43));
    assert_eq!(updated.products.len(), 1);
    assert_eq!(
        updated.products[0].description.as_deref(),
        Some("Sliced apple")
    );

    let err = repo
        .update_order(
            order.id,
            2,
            &UpdateOrder::new().status(OrderStatus::Completed),
        )
        .expect_err("expected cross-hub update to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let (total_after_update, orders_after_update) = repo
        .list_orders(OrderListQuery::new(1).paginate(1, 10))
        .expect("failed to paginate");
    assert_eq!(total_after_update, 1);
    assert_eq!(orders_after_update[0].status, OrderStatus::Processing);

    let err = repo
        .delete_order(order.id, 2)
        .expect_err("expected cross-hub delete to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_order(order.id, 1)
        .expect("failed to delete order");
    assert!(
        repo.get_order_by_id(order.id, 1)
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_final, orders_final) = repo
        .list_orders(OrderListQuery::new(1))
        .expect("failed to list after delete");
    assert_eq!(total_final, 0);
    assert!(orders_final.is_empty());
}
