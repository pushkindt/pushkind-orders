use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{
    App, HttpResponse,
    http::{StatusCode, header},
    test, web,
};
use pushkind_orders::domain::{
    category::NewCategory, customer::Customer, customer::NewCustomer, order::NewOrder,
    order::OrderProduct, price_level::NewPriceLevel, product::NewProduct,
    product_price_level::NewProductPriceLevelRate, tag::NewTag,
};
use pushkind_orders::repository::{
    CategoryWriter, CustomerWriter, DieselRepository, OrderWriter, PriceLevelWriter, ProductWriter,
    TagWriter,
};
use pushkind_orders::routes::store::{
    create_store_order_handler, get_store_product, list_store_categories,
    list_store_orders_handler, list_store_products, list_store_tags,
};
use pushkind_orders::routes::store_session::set_store_customer;
use pushkind_orders::services::store::{StoreCategory, StoreOrder, StoreProduct, StoreTag};
use serde_json::json;

mod common;

async fn set_session_customer(session: Session, customer: web::Data<Customer>) -> HttpResponse {
    match set_store_customer(&session, &customer) {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::test]
async fn store_endpoints_return_data() {
    let test_db = common::TestDb::new("routes_store_endpoints_return_data.db");
    let repo = DieselRepository::new(test_db.pool());

    let _category = repo
        .create_category(&NewCategory::new(1, "Beverages"))
        .expect("create category");
    let tag = repo
        .create_tag(&NewTag::try_new(1, "Organic").expect("build tag"))
        .expect("create tag");

    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD"))
        .expect("create product");
    repo.replace_product_tags(product.id, 1, &[tag.id.get()])
        .expect("attach tag");

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
                .service(get_store_product)
                .service(list_store_products)
                .service(list_store_categories)
                .service(list_store_tags),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/products")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let products: Vec<StoreProduct> = test::read_body_json(resp).await;
    assert_eq!(products.len(), 1);
    assert_eq!(products[0].name, "Coffee");
    assert_eq!(products[0].tags.len(), 1);
    assert_eq!(products[0].tags[0].name, "Organic");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/store/1/products/{}", product.id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let product_response: StoreProduct = test::read_body_json(resp).await;
    assert_eq!(product_response.id, product.id);
    assert_eq!(product_response.name, "Coffee");

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/categories")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let categories: Vec<StoreCategory> = test::read_body_json(resp).await;
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].name, "Beverages");

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/tags")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let tags: Vec<StoreTag> = test::read_body_json(resp).await;
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "Organic");
}

#[actix_web::test]
async fn store_products_respect_query_parameters() {
    let test_db = common::TestDb::new("routes_store_query_params.db");
    let repo = DieselRepository::new(test_db.pool());

    let beverages = repo
        .create_category(&NewCategory::new(1, "Beverages"))
        .expect("create beverages category");
    let _snacks = repo
        .create_category(&NewCategory::new(1, "Snacks"))
        .expect("create snacks category");

    repo.create_product(&NewProduct::new(1, "Coffee Beans", "USD").with_category_id(beverages.id))
        .expect("create coffee product");
    repo.create_product(&NewProduct::new(1, "Special Tea", "USD"))
        .expect("create tea product");

    for index in 0..21 {
        repo.create_product(&NewProduct::new(1, format!("Extra Item {index}"), "USD"))
            .expect("create extra product");
    }

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
                .service(get_store_product)
                .service(list_store_products)
                .service(list_store_categories)
                .service(list_store_tags),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/products")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let default_products: Vec<StoreProduct> = test::read_body_json(resp).await;
    assert_eq!(default_products.len(), 22);
    assert!(
        default_products
            .iter()
            .all(|product| product.category_id.is_none())
    );

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/store/1/products?categoryId={}",
            beverages.id
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let category_products: Vec<StoreProduct> = test::read_body_json(resp).await;
    assert!(!category_products.is_empty());
    assert!(
        category_products
            .iter()
            .all(|product| product.category_id == Some(beverages.id))
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/products?search=Tea")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let search_products: Vec<StoreProduct> = test::read_body_json(resp).await;
    assert_eq!(search_products.len(), 1);
    assert!(search_products[0].name.contains("Tea"));

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/products?page=2")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let paginated_products: Vec<StoreProduct> = test::read_body_json(resp).await;
    assert!(paginated_products.len() <= pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE);
    assert!(
        paginated_products
            .iter()
            .all(|product| product.category_id.is_none())
    );
}

#[actix_web::test]
async fn store_categories_respect_parent_query_parameter() {
    let test_db = common::TestDb::new("routes_store_category_query.db");
    let repo = DieselRepository::new(test_db.pool());

    let beverages = repo
        .create_category(&NewCategory::new(1, "Beverages"))
        .expect("create root category");
    repo.create_category(&NewCategory::new(1, "Coffee").with_parent_id(beverages.id))
        .expect("create child category");

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
                .service(get_store_product)
                .service(list_store_products)
                .service(list_store_categories)
                .service(list_store_tags),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/categories")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let root_categories: Vec<StoreCategory> = test::read_body_json(resp).await;
    assert_eq!(root_categories.len(), 1);
    assert_eq!(root_categories[0].id, beverages.id);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/store/1/categories?parentId={}",
            beverages.id
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let child_categories: Vec<StoreCategory> = test::read_body_json(resp).await;
    assert_eq!(child_categories.len(), 1);
    assert_eq!(child_categories[0].parent_id, Some(beverages.id));
}

#[actix_web::test]
async fn store_routes_reject_invalid_hub_id() {
    let test_db = common::TestDb::new("routes_store_invalid_hub_id.db");
    let repo = DieselRepository::new(test_db.pool());

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
                .service(get_store_product)
                .service(list_store_products)
                .service(list_store_categories)
                .service(list_store_tags),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/abc/products")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/products/xyz")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::get()
        .uri("/api/v1/store/abc/categories")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::get()
        .uri("/api/v1/store/abc/tags")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn store_product_returns_not_found_for_unknown_id() {
    let test_db = common::TestDb::new("routes_store_product_not_found.db");
    let repo = DieselRepository::new(test_db.pool());

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
                .service(get_store_product)
                .service(list_store_products)
                .service(list_store_categories)
                .service(list_store_tags),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/products/42")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn create_store_order_requires_authentication() {
    let test_db = common::TestDb::new("routes_store_order_requires_auth.db");
    let repo = DieselRepository::new(test_db.pool());
    let price_level = repo
        .create_price_level(&NewPriceLevel::new(1, "Default", true))
        .expect("create price level");
    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD"))
        .expect("create product");
    repo.replace_product_price_levels(
        product.id,
        1,
        &[NewProductPriceLevelRate::new(
            product.id,
            price_level.id,
            500,
        )],
    )
    .expect("attach price level");

    let key = Key::generate();
    let app_repo = repo.clone();
    let app = test::init_service(
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key)
                    .cookie_name("store-session".to_string())
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(app_repo))
            .service(web::scope("/api/v1/store").service(create_store_order_handler)),
    )
    .await;

    let request_body = json!([{ "productId": product.id, "quantity": 1 }]);
    let req = test::TestRequest::post()
        .uri("/api/v1/store/1/orders")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload(request_body.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn create_store_order_validates_payload() {
    let test_db = common::TestDb::new("routes_store_order_validates_payload.db");
    let repo = DieselRepository::new(test_db.pool());
    let price_level = repo
        .create_price_level(&NewPriceLevel::new(1, "Default", true))
        .expect("create price level");
    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD"))
        .expect("create product");
    repo.replace_product_price_levels(
        product.id,
        1,
        &[NewProductPriceLevelRate::new(
            product.id,
            price_level.id,
            500,
        )],
    )
    .expect("attach price level");
    let customer = repo
        .create_customer(&NewCustomer::new(1, "John", "+111"))
        .expect("create customer");

    let key = Key::generate();
    let app_repo = repo.clone();
    let app = test::init_service(
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key.clone())
                    .cookie_name("store-session".to_string())
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(app_repo))
            .app_data(web::Data::new(customer.clone()))
            .service(
                web::scope("/api/v1/store")
                    .service(create_store_order_handler)
                    .service(web::resource("/login").route(web::post().to(set_session_customer))),
            ),
    )
    .await;

    let login_req = test::TestRequest::post()
        .uri("/api/v1/store/login")
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);
    let cookie = login_resp
        .response()
        .cookies()
        .next()
        .expect("session cookie");
    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    let request_body = json!([{ "productId": product.id, "quantity": 0 }]);
    let req = test::TestRequest::post()
        .uri("/api/v1/store/1/orders")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::COOKIE, cookie_header))
        .set_payload(request_body.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[actix_web::test]
async fn create_store_order_creates_order() {
    let test_db = common::TestDb::new("routes_store_order_creates.db");
    let repo = DieselRepository::new(test_db.pool());
    let price_level = repo
        .create_price_level(&NewPriceLevel::new(1, "Default", true))
        .expect("create price level");
    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD"))
        .expect("create product");
    repo.replace_product_price_levels(
        product.id,
        1,
        &[NewProductPriceLevelRate::new(
            product.id,
            price_level.id,
            500,
        )],
    )
    .expect("attach price level");
    let customer = repo
        .create_customer(&NewCustomer::new(1, "John", "+111"))
        .expect("create customer");

    let key = Key::generate();
    let app_repo = repo.clone();
    let app_customer = customer.clone();
    let app = test::init_service(
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key.clone())
                    .cookie_name("store-session".to_string())
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(app_repo))
            .app_data(web::Data::new(app_customer))
            .service(
                web::scope("/api/v1/store")
                    .service(create_store_order_handler)
                    .service(web::resource("/login").route(web::post().to(set_session_customer))),
            ),
    )
    .await;

    let login_req = test::TestRequest::post()
        .uri("/api/v1/store/login")
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);
    let cookie = login_resp
        .response()
        .cookies()
        .next()
        .expect("session cookie");
    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    let request_body = json!([{ "productId": product.id, "quantity": 2 }]);
    let req = test::TestRequest::post()
        .uri("/api/v1/store/1/orders")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::COOKIE, cookie_header))
        .set_payload(request_body.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let order: StoreOrder = test::read_body_json(resp).await;
    assert_eq!(order.customer_id, Some(customer.id));
    assert_eq!(order.total_cents, 1000);
    assert_eq!(order.products.len(), 1);
    assert_eq!(order.products[0].price_cents, 1000);
}

#[actix_web::test]
async fn list_store_orders_requires_authentication() {
    let test_db = common::TestDb::new("routes_store_orders_requires_auth.db");
    let repo = DieselRepository::new(test_db.pool());

    let key = Key::generate();
    let app_repo = repo.clone();
    let app = test::init_service(
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key)
                    .cookie_name("store-session".to_string())
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(app_repo))
            .service(web::scope("/api/v1/store").service(list_store_orders_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/orders")
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn list_store_orders_returns_orders_for_customer() {
    let test_db = common::TestDb::new("routes_store_orders_returns_data.db");
    let repo = DieselRepository::new(test_db.pool());
    let customer = repo
        .create_customer(&NewCustomer::new(1, "Customer", "+111"))
        .expect("create customer");

    let product = OrderProduct::new("Coffee", 500, "USD", 1).with_product_id(1);
    let new_order = NewOrder::new(1, 500, "USD")
        .with_customer_id(customer.id)
        .with_products(vec![product]);
    repo.create_order(&new_order).expect("create order");

    let key = Key::generate();
    let app_repo = repo.clone();
    let app_customer = customer.clone();
    let app = test::init_service(
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key.clone())
                    .cookie_name("store-session".to_string())
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(app_repo))
            .app_data(web::Data::new(app_customer))
            .service(
                web::scope("/api/v1/store")
                    .service(list_store_orders_handler)
                    .service(web::resource("/login").route(web::post().to(set_session_customer))),
            ),
    )
    .await;

    let login_req = test::TestRequest::post()
        .uri("/api/v1/store/login")
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);
    let cookie = login_resp
        .response()
        .cookies()
        .next()
        .expect("session cookie");
    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/orders")
        .insert_header((header::COOKIE, cookie_header))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let orders: Vec<StoreOrder> = test::read_body_json(resp).await;
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].customer_id, Some(customer.id));
    assert_eq!(orders[0].total_cents, 500);
}

#[actix_web::test]
async fn list_store_orders_returns_empty_results() {
    let test_db = common::TestDb::new("routes_store_orders_empty_results.db");
    let repo = DieselRepository::new(test_db.pool());
    let customer = repo
        .create_customer(&NewCustomer::new(1, "Customer", "+111"))
        .expect("create customer");

    let key = Key::generate();
    let app_repo = repo.clone();
    let app_customer = customer.clone();
    let app = test::init_service(
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key.clone())
                    .cookie_name("store-session".to_string())
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(app_repo))
            .app_data(web::Data::new(app_customer))
            .service(
                web::scope("/api/v1/store")
                    .service(list_store_orders_handler)
                    .service(web::resource("/login").route(web::post().to(set_session_customer))),
            ),
    )
    .await;

    let login_req = test::TestRequest::post()
        .uri("/api/v1/store/login")
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);
    let cookie = login_resp
        .response()
        .cookies()
        .next()
        .expect("session cookie");
    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    let req = test::TestRequest::get()
        .uri("/api/v1/store/1/orders")
        .insert_header((header::COOKIE, cookie_header))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let orders: Vec<StoreOrder> = test::read_body_json(resp).await;
    assert!(orders.is_empty());
}
