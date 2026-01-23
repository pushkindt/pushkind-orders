use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryError;
use pushkind_orders::domain::{
    category::NewCategory as DomainNewCategory,
    customer::CustomerListQuery,
    customer::NewCustomer,
    order::{NewOrder, OrderListQuery, OrderProduct, OrderStatus, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, ProductListQuery, UpdateProduct},
    product_price_level::NewProductPriceLevelRate,
    types::{
        CategoryId, CategoryName, CurrencyCode, CustomerId, HubId, OrderNotes, OrderReference,
        PhoneNumber, PriceCents, PriceLevelName, ProductDescription, ProductName, ProductSku,
        UserEmail,
    },
    user::{NewUser, UpdateUser},
    vendor::NewVendor,
};
use pushkind_orders::models::category::NewCategory as DbNewCategory;
use pushkind_orders::models::product_price_level::NewProductPriceLevel as DbNewProductPriceLevel;
use pushkind_orders::repository::DieselRepository;
use pushkind_orders::repository::{
    CustomerReader, CustomerWriter, OrderReader, OrderWriter, PriceLevelReader, PriceLevelWriter,
    ProductReader, ProductWriter, UserListQuery, UserReader, UserWriter, VendorOrderWriter,
    VendorWriter,
};
use pushkind_orders::schema::categories;

mod common;

#[test]
fn test_user_repository_crud() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let alice_new =
        NewUser::try_new(1, "Alice".to_string(), "alice@example.com".to_string()).unwrap();
    let bob_new = NewUser::try_new(1, "Bob".to_string(), "bob@example.com".to_string()).unwrap();

    let alice = repo
        .create_user(&alice_new)
        .expect("failed to create Alice");
    let bob = repo.create_user(&bob_new).expect("failed to create Bob");

    assert_eq!(alice.name.as_str(), "Alice");
    assert_eq!(alice.email.as_str(), "alice@example.com");

    let hub_id = HubId::new(1).expect("valid hub id");
    let fetched = repo
        .get_user_by_id(alice.id, hub_id)
        .expect("failed to fetch user")
        .expect("expected Alice to exist");
    assert_eq!(fetched.id, alice.id);

    assert!(
        repo.get_user_by_id(alice.id, HubId::new(2).expect("valid hub id"))
            .expect("failed to fetch scoped user")
            .is_none()
    );

    let fetched_by_email = repo
        .get_user_by_email(&alice.email, hub_id)
        .expect("failed to fetch by email")
        .expect("expected Alice via email");
    assert_eq!(fetched_by_email.id, alice.id);

    assert!(
        repo.get_user_by_email(
            &UserEmail::new("alice@example.com").expect("valid email"),
            HubId::new(2).expect("valid hub id"),
        )
        .expect("failed to fetch by email scoped")
        .is_none()
    );

    let (total_all, users_all) = repo
        .list_users(UserListQuery::new(hub_id))
        .expect("failed to list users");
    assert_eq!(total_all, 2);
    assert_eq!(users_all.len(), 2);

    let (total_filtered, users_filtered) = repo
        .list_users(UserListQuery::new(hub_id).search("bob"))
        .expect("failed to search users");
    assert_eq!(total_filtered, 1);
    assert_eq!(users_filtered[0].id, bob.id);

    let updates = UpdateUser::try_new("Alicia".to_string()).expect("failed to build update");

    let updated = repo
        .update_user(alice.id, hub_id, &updates)
        .expect("failed to update user");
    assert_eq!(updated.name.as_str(), "Alicia");

    let err = repo
        .update_user(alice.id, HubId::new(2).expect("valid hub id"), &updates)
        .expect_err("expected cross-hub update to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .delete_user(alice.id, HubId::new(2).expect("valid hub id"))
        .expect_err("expected cross-hub delete to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_user(alice.id, hub_id)
        .expect("failed to delete user");
    assert!(
        repo.get_user_by_id(alice.id, hub_id)
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_after, users_after) = repo
        .list_users(UserListQuery::new(hub_id))
        .expect("failed to list after delete");
    assert_eq!(total_after, 1);
    assert_eq!(users_after[0].id, bob.id);
}

#[test]
fn test_customer_repository_crud() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let vip_level = repo
        .create_price_level(&NewPriceLevel::try_new(1, "VIP", false).unwrap())
        .expect("failed to create price level");

    let alice_new = NewCustomer::try_new(1, "Alice", "+15551234").expect("valid alice");
    let bob_new = NewCustomer::try_new(1, "Bob", "+15550000").expect("valid bob");
    let carla_new = NewCustomer::try_new(2, "Carla", "+18880000").expect("valid carla");

    let alice = repo
        .create_customer(&alice_new)
        .expect("failed to create Alice");
    let bob = repo
        .create_customer(&bob_new)
        .expect("failed to create Bob");
    let carla = repo
        .create_customer(&carla_new)
        .expect("failed to create Carla");

    assert_eq!(alice.phone.as_str(), "+15551234");
    assert_eq!(bob.price_level_id, None);
    assert_eq!(carla.hub_id.get(), 2);

    let hub_id = HubId::new(1).expect("valid hub id");
    let fetched = repo
        .get_customer_by_id(alice.id, hub_id)
        .expect("failed to fetch customer")
        .expect("expected Alice to exist");
    assert_eq!(fetched.id, alice.id);
    assert_eq!(fetched.phone.as_str(), "+15551234");

    assert!(
        repo.get_customer_by_id(alice.id, HubId::new(2).expect("valid hub id"))
            .expect("failed to fetch scoped customer")
            .is_none()
    );

    let fetched_by_phone = repo
        .get_customer_by_phone(&PhoneNumber::new("+15551234").expect("valid phone"), hub_id)
        .expect("failed to fetch by phone")
        .expect("expected Alice via contact");
    assert_eq!(fetched_by_phone.id, alice.id);

    assert!(
        repo.get_customer_by_phone(&PhoneNumber::new("+15559999").expect("valid phone"), hub_id,)
            .expect("failed to fetch missing phone")
            .is_none()
    );

    assert!(
        repo.get_customer_by_phone(
            &PhoneNumber::new("+15551234").expect("valid phone"),
            HubId::new(2).expect("valid hub id"),
        )
        .expect("failed to fetch scoped phone")
        .is_none()
    );

    let fetched_bob_by_phone = repo
        .get_customer_by_phone(&PhoneNumber::new("+15550000").expect("valid phone"), hub_id)
        .expect("failed to fetch bob by phone")
        .expect("expected Bob via phone");
    assert_eq!(fetched_bob_by_phone.id, bob.id);

    let (total_all, customers_all) = repo
        .list_customers(CustomerListQuery::try_new(1).expect("valid hub id"))
        .expect("failed to list customers");
    assert_eq!(total_all, 2);
    assert_eq!(customers_all.len(), 2);

    let (total_filtered, customers_filtered) = repo
        .list_customers(
            CustomerListQuery::try_new(1)
                .expect("valid hub")
                .search("bob"),
        )
        .expect("failed to search customers");
    assert_eq!(total_filtered, 1);
    assert_eq!(customers_filtered[0].id, bob.id);

    repo.assign_price_level_to_customers(hub_id, &[alice.id], Some(vip_level.id))
        .expect("failed to assign price level");

    let (total_vip, vip_customers) = repo
        .list_customers(
            CustomerListQuery::try_new(1)
                .expect("valid hub")
                .price_level(vip_level.id),
        )
        .expect("failed to list vip customers");
    assert_eq!(total_vip, 1);
    assert_eq!(vip_customers[0].id, alice.id);
    assert_eq!(vip_customers[0].price_level_id, Some(vip_level.id));

    let updated = repo
        .get_customer_by_id(alice.id, hub_id)
        .expect("failed to fetch after assignment")
        .expect("expected Alice after assignment");
    assert_eq!(updated.price_level_id, Some(vip_level.id));

    repo.assign_price_level_to_customers(hub_id, &[alice.id], None)
        .expect("failed to clear price level");

    let cleared = repo
        .get_customer_by_id(alice.id, hub_id)
        .expect("failed to fetch after clearing")
        .expect("expected Alice after clearing");
    assert_eq!(cleared.price_level_id, None);

    let err = repo
        .assign_price_level_to_customers(
            hub_id,
            &[CustomerId::new(9999).expect("valid id")],
            Some(vip_level.id),
        )
        .expect_err("expected assigning missing customer to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .assign_price_level_to_customers(
            HubId::new(2).expect("valid hub id"),
            &[carla.id],
            Some(vip_level.id),
        )
        .expect_err("expected cross-hub price level to fail");
    assert!(matches!(err, RepositoryError::NotFound));
}

#[test]
fn test_product_repository_crud() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let mut conn = test_db.pool().get().expect("obtain connection");
    let category_domain =
        DomainNewCategory::new(HubId::new(1).unwrap(), CategoryName::new("Fruit").unwrap());
    let db_category = DbNewCategory::from(&category_domain);
    let category_id: i32 = diesel::insert_into(categories::table)
        .values(&db_category)
        .returning(categories::id)
        .get_result(&mut conn)
        .expect("create category");

    let apple_new = NewProduct::try_new(1, "Apple", "USD")
        .unwrap()
        .with_sku(ProductSku::new("APL-1").unwrap())
        .with_description(ProductDescription::new("Fresh apple").unwrap())
        .with_category_id(CategoryId::new(category_id).unwrap());
    let banana_new = NewProduct::try_new(1, "Banana", "USD").unwrap();

    let apple = repo
        .create_product(&apple_new)
        .expect("failed to create apple product");
    let banana = repo
        .create_product(&banana_new)
        .expect("failed to create banana product");

    assert_eq!(apple.name, "Apple");
    assert_eq!(apple.sku.as_deref(), Some("APL-1"));
    assert_eq!(apple.category_id.map(|id| id.get()), Some(category_id));
    assert!(apple.price_levels.is_empty());
    assert!(banana.price_levels.is_empty());

    let err = repo
        .create_product(
            &NewProduct::try_new(1, "Ghost", "USD")
                .unwrap()
                .with_category_id(CategoryId::new(category_id + 999).unwrap()),
        )
        .expect_err("expected missing category");
    assert!(matches!(err, RepositoryError::NotFound));

    let hub_id = HubId::new(1).expect("valid hub id");
    let fetched = repo
        .get_product_by_id(apple.id, hub_id)
        .expect("failed to fetch by id")
        .expect("expected apple product");
    assert_eq!(fetched.id, apple.id);
    assert!(fetched.price_levels.is_empty());

    assert!(
        repo.get_product_by_id(apple.id, HubId::new(2).expect("valid hub id"))
            .expect("failed to fetch cross-hub")
            .is_none()
    );

    let (total_all, products_all) = repo
        .list_products(ProductListQuery::try_new(1).unwrap())
        .expect("failed to list products");
    assert_eq!(total_all, 2);
    assert_eq!(products_all.len(), 2);
    assert!(
        products_all
            .iter()
            .all(|product| product.price_levels.is_empty())
    );

    let (total_search, products_search) = repo
        .list_products(ProductListQuery::try_new(1).unwrap().search("apple"))
        .expect("failed to search products");
    assert_eq!(total_search, 1);
    assert_eq!(products_search[0].id, apple.id);
    assert!(products_search[0].price_levels.is_empty());

    let (total_sku, products_sku) = repo
        .list_products(
            ProductListQuery::try_new(1)
                .unwrap()
                .sku(ProductSku::new("APL-1").unwrap()),
        )
        .expect("failed to list by sku");
    assert_eq!(total_sku, 1);
    assert_eq!(products_sku[0].id, apple.id);

    let updates = UpdateProduct::new(
        ProductName::new("Apple Premium").unwrap(),
        CurrencyCode::new("USD").unwrap(),
        true,
    );

    let updated = repo
        .update_product(apple.id, hub_id, &updates)
        .expect("failed to update product");
    assert!(updated.is_archived);
    assert_eq!(updated.name, "Apple Premium");
    assert!(updated.price_levels.is_empty());

    let updates = UpdateProduct::new(
        ProductName::new("Apple").unwrap(),
        CurrencyCode::new("USD").unwrap(),
        false,
    );

    let err = repo
        .update_product(apple.id, HubId::new(2).expect("valid hub id"), &updates)
        .expect_err("expected cross-hub update failure");
    assert!(matches!(err, RepositoryError::NotFound));

    let (total_visible, products_visible) = repo
        .list_products(ProductListQuery::try_new(1).unwrap())
        .expect("failed to list non-archived");
    assert_eq!(total_visible, 1);
    assert_eq!(products_visible[0].id, banana.id);

    let (total_with_archived, products_with_archived) = repo
        .list_products(ProductListQuery::try_new(1).unwrap().include_archived())
        .expect("failed to list including archived");
    assert_eq!(total_with_archived, 2);
    assert_eq!(products_with_archived.len(), 2);

    let err = repo
        .delete_product(apple.id, HubId::new(2).expect("valid hub id"))
        .expect_err("expected cross-hub delete failure");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_product(apple.id, hub_id)
        .expect("failed to delete product");
    assert!(
        repo.get_product_by_id(apple.id, hub_id)
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_final, products_final) = repo
        .list_products(ProductListQuery::try_new(1).unwrap().include_archived())
        .expect("failed final list");
    assert_eq!(total_final, 1);
    assert_eq!(products_final[0].id, banana.id);
}

#[test]
fn test_product_repository_vendor_filter() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let vendor = repo
        .create_vendor(&NewVendor::try_new(1, "Fresh Produce").unwrap())
        .expect("failed to create vendor");

    let vendor_product = repo
        .create_product(
            &NewProduct::try_new(1, "Vendor Apple", "USD")
                .unwrap()
                .with_vendor_id(vendor.id),
        )
        .expect("failed to create vendor product");

    repo.create_product(&NewProduct::try_new(1, "Generic Banana", "USD").unwrap())
        .expect("failed to create generic product");

    let (total, filtered) = repo
        .list_products(
            ProductListQuery::try_new(1)
                .unwrap()
                .with_vendor_id(vendor.id),
        )
        .expect("failed to list by vendor");

    assert_eq!(total, 1);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, vendor_product.id);
    assert_eq!(filtered[0].vendor_id, Some(vendor.id));
}

#[test]
fn test_replace_product_price_levels() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).expect("valid hub id");

    let retail_level = repo
        .create_price_level(&NewPriceLevel::try_new(1, "Retail", false).unwrap())
        .expect("failed to create price level");
    let wholesale_level = repo
        .create_price_level(&NewPriceLevel::try_new(1, "Wholesale", false).unwrap())
        .expect("failed to create price level");

    let product = repo
        .create_product(&NewProduct::try_new(1, "Coffee", "USD").unwrap())
        .expect("failed to create product");

    let rates = vec![
        NewProductPriceLevelRate::new(product.id, retail_level.id, PriceCents::new(1250).unwrap()),
        NewProductPriceLevelRate::new(
            product.id,
            wholesale_level.id,
            PriceCents::new(990).unwrap(),
        ),
    ];

    repo.replace_product_price_levels(product.id, hub_id, &rates)
        .expect("failed to replace product price levels");

    let mut fetched = repo
        .get_product_by_id(product.id, hub_id)
        .expect("failed to fetch product")
        .expect("product should exist");

    fetched
        .price_levels
        .sort_by_key(|rate| rate.price_level_id.get());

    assert_eq!(fetched.price_levels.len(), 2);
    assert_eq!(fetched.price_levels[0].price_level_id, retail_level.id);
    assert_eq!(fetched.price_levels[0].price_cents.get(), 1250);
    assert_eq!(fetched.price_levels[1].price_level_id, wholesale_level.id);
    assert_eq!(fetched.price_levels[1].price_cents.get(), 990);

    let err = repo
        .replace_product_price_levels(product.id, HubId::new(2).expect("valid hub id"), &rates)
        .expect_err("expected cross-hub update to fail");
    assert!(matches!(err, RepositoryError::NotFound));
}

#[test]
fn test_price_level_repository_crud() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let bronze_new = NewPriceLevel::new(
        HubId::new(1).unwrap(),
        PriceLevelName::new("Bronze").unwrap(),
        false,
    );
    let silver_new = NewPriceLevel::new(
        HubId::new(1).unwrap(),
        PriceLevelName::new("Silver").unwrap(),
        false,
    );

    let bronze = repo
        .create_price_level(&bronze_new)
        .expect("failed to create bronze level");
    let silver = repo
        .create_price_level(&silver_new)
        .expect("failed to create silver level");

    assert_eq!(bronze.name.as_str(), "Bronze");
    assert_eq!(silver.name.as_str(), "Silver");
    assert!(
        bronze.is_default,
        "first price level should default to true"
    );
    assert!(
        !silver.is_default,
        "subsequent price level should respect provided default flag"
    );

    let fetched = repo
        .get_price_level_by_id(bronze.id, HubId::new(1).expect("valid hub id"))
        .expect("failed to fetch by id")
        .expect("expected bronze price level");
    assert_eq!(fetched.id, bronze.id);
    assert_eq!(fetched.name.as_str(), "Bronze");

    assert!(
        repo.get_price_level_by_id(bronze.id, HubId::new(2).expect("valid hub id"))
            .expect("failed to fetch cross-hub")
            .is_none()
    );

    let (total_all, levels_all) = repo
        .list_price_levels(PriceLevelListQuery::try_new(1).unwrap())
        .expect("failed to list price levels");
    assert_eq!(total_all, 2);
    assert_eq!(levels_all.len(), 2);

    let (total_search, levels_search) = repo
        .list_price_levels(PriceLevelListQuery::try_new(1).unwrap().search("Sil"))
        .expect("failed to search price levels");
    assert_eq!(total_search, 1);
    assert_eq!(levels_search[0].id, silver.id);

    let updates = UpdatePriceLevel::new(PriceLevelName::new("Gold").unwrap(), false);

    let updated = repo
        .update_price_level(bronze.id, HubId::new(1).expect("valid hub id"), &updates)
        .expect("failed to update price level");
    assert_eq!(updated.name.as_str(), "Gold");

    let cross_hub_updates = UpdatePriceLevel::new(PriceLevelName::new("Intruder").unwrap(), false);

    let err = repo
        .update_price_level(
            bronze.id,
            HubId::new(2).expect("valid hub id"),
            &cross_hub_updates,
        )
        .expect_err("expected cross-hub update failure");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .delete_price_level(bronze.id, HubId::new(2).expect("valid hub id"))
        .expect_err("expected cross-hub delete failure");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_price_level(bronze.id, HubId::new(1).expect("valid hub id"))
        .expect("failed to delete price level");
    assert!(
        repo.get_price_level_by_id(bronze.id, HubId::new(1).expect("valid hub id"))
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_final, levels_final) = repo
        .list_price_levels(PriceLevelListQuery::try_new(1).unwrap())
        .expect("failed to list after delete");
    assert_eq!(total_final, 1);
    assert_eq!(levels_final[0].id, silver.id);
}

#[test]
fn updating_price_level_default_resets_previous_default() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let original_default = repo
        .create_price_level(&NewPriceLevel::try_new(1, "Default", false).unwrap())
        .expect("failed to create initial default level");
    let secondary = repo
        .create_price_level(&NewPriceLevel::try_new(1, "Secondary", false).unwrap())
        .expect("failed to create secondary level");

    assert!(
        original_default.is_default,
        "expected first level to be default"
    );
    assert!(
        !secondary.is_default,
        "expected second level to respect payload flag"
    );

    let updates = UpdatePriceLevel {
        name: secondary.name.clone(),
        updated_at: chrono::Utc::now().naive_utc(),
        is_default: true,
    };

    let updated = repo
        .update_price_level(secondary.id, HubId::new(1).expect("valid hub id"), &updates)
        .expect("failed to promote second level to default");

    assert!(updated.is_default, "expected updated level to be default");

    let demoted = repo
        .get_price_level_by_id(original_default.id, HubId::new(1).expect("valid hub id"))
        .expect("failed to fetch original default")
        .expect("expected original level to exist after update");

    assert!(
        !demoted.is_default,
        "expected repository to clear default flag on other levels",
    );
}

#[test]
fn deleting_price_level_removes_product_rates() {
    use diesel::prelude::*;
    use pushkind_orders::schema::product_price_levels::dsl as product_rates;

    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let product = repo
        .create_product(&NewProduct::try_new(1, "Cascade Product", "USD").unwrap())
        .expect("failed to create product");
    let price_level = repo
        .create_price_level(&NewPriceLevel::try_new(1, "Cascade Level", false).unwrap())
        .expect("failed to create price level");

    {
        let mut conn = test_db
            .pool()
            .get()
            .expect("failed to acquire connection for insert");

        let new_rate = DbNewProductPriceLevel {
            product_id: product.id.get(),
            price_level_id: price_level.id.get(),
            price_cents: 1950,
        };

        diesel::insert_into(product_rates::product_price_levels)
            .values(&new_rate)
            .execute(&mut conn)
            .expect("failed to insert product price level");

        let existing: i64 = product_rates::product_price_levels
            .filter(product_rates::price_level_id.eq(price_level.id.get()))
            .count()
            .get_result(&mut conn)
            .expect("failed to count inserted rates");
        assert_eq!(existing, 1);
    }

    repo.delete_price_level(price_level.id, HubId::new(1).expect("valid hub id"))
        .expect("failed to delete price level");

    {
        let mut conn = test_db
            .pool()
            .get()
            .expect("failed to acquire connection for verification");
        let remaining: i64 = product_rates::product_price_levels
            .filter(product_rates::product_id.eq(product.id.get()))
            .count()
            .get_result(&mut conn)
            .expect("failed to count remaining rates");
        assert_eq!(remaining, 0, "expected cascade delete to remove rates");
    }

    let updated_product = repo
        .get_product_by_id(product.id, HubId::new(1).expect("valid hub id"))
        .expect("failed to fetch product after cascade")
        .expect("product should still exist");
    assert!(
        updated_product.price_levels.is_empty(),
        "product should have no price levels after cascade delete"
    );
}

#[test]
fn test_order_repository_crud() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let product_snapshot = OrderProduct::try_new("Apple", 150, "USD", 2, None)
        .unwrap()
        .with_sku(ProductSku::new("APL-1").unwrap())
        .with_description(ProductDescription::new("Fresh apple").unwrap());

    let new_order = NewOrder::try_new(1, 300, "USD")
        .unwrap()
        .with_reference(OrderReference::new("REF-001").unwrap())
        .with_notes(OrderNotes::new("Handle with care").unwrap())
        .with_status(OrderStatus::Pending)
        .with_products(vec![product_snapshot.clone()]);

    let order = repo
        .create_order(&new_order)
        .expect("failed to create order");
    assert_eq!(order.hub_id.get(), 1);
    assert_eq!(order.status, OrderStatus::Pending);
    assert_eq!(order.products.len(), 1);
    assert_eq!(order.products[0].name.as_str(), "Apple");

    let fetched = repo
        .get_order_by_id(order.id, HubId::new(1).unwrap())
        .expect("failed to fetch order")
        .expect("order should exist");
    assert_eq!(fetched.id, order.id);
    assert_eq!(fetched.products.len(), 1);

    assert!(
        repo.get_order_by_id(order.id, HubId::new(2).unwrap())
            .expect("failed scoped fetch")
            .is_none()
    );

    let (total_all, orders_all) = repo
        .list_orders(OrderListQuery::try_new(1).unwrap())
        .expect("failed to list orders");
    assert_eq!(total_all, 1);
    assert_eq!(orders_all.len(), 1);

    let (total_status, orders_status) = repo
        .list_orders(
            OrderListQuery::try_new(1)
                .unwrap()
                .status(OrderStatus::Pending),
        )
        .expect("failed to filter by status");
    assert_eq!(total_status, 1);
    assert_eq!(orders_status[0].id, order.id);

    let (total_search, orders_search) = repo
        .list_orders(OrderListQuery::try_new(1).unwrap().search("REF-001"))
        .expect("failed to search orders");
    assert_eq!(total_search, 1);
    assert_eq!(orders_search[0].id, order.id);

    let (total_none, _) = repo
        .list_orders(OrderListQuery::try_new(1).unwrap().search("missing"))
        .expect("failed to search missing");
    assert_eq!(total_none, 0);

    let updates = UpdateOrder {
        status: OrderStatus::Processing,
        notes: Some(OrderNotes::new("Pack immediately").unwrap()),
        reference: order.reference.clone(),
        updated_at: chrono::Utc::now().naive_utc(),
        shipping_address: None,
        consignee: None,
        delivery_notes: None,
        payer: None,
    };

    let updated = repo
        .update_order(order.id, HubId::new(1).unwrap(), &updates)
        .expect("failed to update order");
    assert_eq!(updated.status, OrderStatus::Processing);

    let mut cross_hub_updates = updates.clone();
    cross_hub_updates.status = OrderStatus::Completed;

    let err = repo
        .update_order(order.id, HubId::new(2).unwrap(), &cross_hub_updates)
        .expect_err("expected cross-hub update to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let (total_after_update, orders_after_update) = repo
        .list_orders(OrderListQuery::try_new(1).unwrap().paginate(1, 10))
        .expect("failed to paginate");
    assert_eq!(total_after_update, 1);
    assert_eq!(orders_after_update[0].status, OrderStatus::Processing);

    let err = repo
        .delete_order(order.id, HubId::new(2).unwrap())
        .expect_err("expected cross-hub delete to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_order(order.id, HubId::new(1).unwrap())
        .expect("failed to delete order");
    assert!(
        repo.get_order_by_id(order.id, HubId::new(1).unwrap())
            .expect("failed to fetch after delete")
            .is_none()
    );

    let (total_final, orders_final) = repo
        .list_orders(OrderListQuery::try_new(1).unwrap())
        .expect("failed to list after delete");
    assert_eq!(total_final, 0);
    assert!(orders_final.is_empty());
}

#[test]
fn test_order_repository_vendor_filter() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let vendor = repo
        .create_vendor(&NewVendor::try_new(1, "Vendor A").unwrap())
        .expect("failed to create vendor");

    let product_snapshot = OrderProduct::try_new("Apple", 150, "USD", 2, None)
        .unwrap()
        .with_sku(ProductSku::new("APL-1").unwrap());

    let vendor_order = repo
        .create_order(
            &NewOrder::try_new(1, 300, "USD")
                .unwrap()
                .with_products(vec![product_snapshot.clone()]),
        )
        .expect("failed to create vendor order");

    repo.associate_order_with_vendor(vendor_order.id, vendor.id, HubId::new(1).unwrap())
        .expect("failed to associate order");

    repo.create_order(
        &NewOrder::try_new(1, 300, "USD")
            .unwrap()
            .with_products(vec![product_snapshot]),
    )
    .expect("failed to create other order");

    let (total, filtered) = repo
        .list_orders(OrderListQuery::try_new(1).unwrap().vendor_id(vendor.id))
        .expect("failed to list orders by vendor");

    assert_eq!(total, 1);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, vendor_order.id);
}
