use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{
    App, HttpMessage, HttpRequest, HttpResponse, Responder,
    http::{StatusCode, header},
    post, test, web,
};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_orders::domain::{
    customer::NewCustomer,
    order::{NewOrder, OrderProduct, OrderStatus},
    price_level::NewPriceLevel,
    product::NewProduct,
    product_price_level::NewProductPriceLevelRate,
    types::{
        HubId, OrderReference, PriceCents, ProductDescription, ProductId, ProductSku, UserEmail,
        UserName, VendorName,
    },
    user::NewUser,
    vendor::NewVendor,
};
use pushkind_orders::models::config::AppConfig;
use pushkind_orders::repository::{
    CustomerWriter, DieselRepository, OrderWriter, PriceLevelWriter, ProductReader, ProductWriter,
    UserWriter, VendorWriter,
};
use pushkind_orders::routes::api::{
    api_v1_assign_vendor_user, api_v1_clear_vendor_user, api_v1_create_local_user,
    api_v1_create_product, api_v1_create_vendor, api_v1_local_users, api_v1_order, api_v1_orders,
    api_v1_product, api_v1_products, api_v1_update_order, api_v1_update_order_product_approvals,
    api_v1_update_product, api_v1_update_vendor, api_v1_upload_products, api_v1_vendor,
    api_v1_vendors,
};
use serde_json::{Value, json};

mod common;

#[derive(serde::Deserialize)]
struct LoginRequest {
    hub_id: i32,
    email: String,
    name: String,
    roles: Vec<String>,
}

#[post("/test/login")]
async fn test_login(
    request: HttpRequest,
    payload: web::Json<LoginRequest>,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    let mut user = AuthenticatedUser {
        sub: payload.email.clone(),
        email: payload.email.clone(),
        hub_id: payload.hub_id,
        name: payload.name.clone(),
        roles: payload.roles.clone(),
        exp: 0,
    };
    user.set_expiration(7);

    let token = user
        .to_jwt(&common_config.secret)
        .expect("JWT generation should succeed for test users.");
    Identity::login(&request.extensions(), token).expect("Test login should persist identity.");

    HttpResponse::Ok().finish()
}

fn common_config() -> CommonServerConfig {
    CommonServerConfig {
        auth_service_url: "https://auth.example.com".to_string(),
        secret: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
    }
}

fn app_config() -> AppConfig {
    AppConfig {
        domain: "example.com".to_string(),
        database_url: "app.db".to_string(),
        secret: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        auth_service_url: "https://auth.example.com".to_string(),
        crm_service_url: "https://crm.example.com".to_string(),
    }
}

fn cookie_header_value<B>(response: &actix_web::dev::ServiceResponse<B>) -> String {
    response
        .response()
        .cookies()
        .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
        .collect::<Vec<_>>()
        .join("; ")
}

fn seed_order(repo: &DieselRepository) -> i32 {
    let customer = repo
        .create_customer(
            &NewCustomer::try_new(1, "John Doe", "+12125550123").expect("valid customer payload"),
        )
        .expect("create customer");

    let order = repo
        .create_order(
            &NewOrder::try_new(1, 5000, "USD")
                .expect("valid order payload")
                .with_customer_id(customer.id)
                .with_reference(OrderReference::new("ORD-101").expect("valid reference"))
                .with_status(OrderStatus::Pending)
                .with_products(vec![
                    OrderProduct::try_new("Coffee", 5000, "USD", 2, Some(6000))
                        .expect("valid product snapshot")
                        .with_product_id(ProductId::new(8).expect("valid product id"))
                        .with_sku(ProductSku::new("COF-1").expect("valid sku")),
                ]),
        )
        .expect("create order");

    order.id.get()
}

fn seed_product(repo: &DieselRepository) -> i32 {
    let hub_id = HubId::new(1).expect("valid hub id");
    let retail = repo
        .create_price_level(&NewPriceLevel::try_new(1, "Retail", false).expect("price level"))
        .expect("create price level");

    let product = repo
        .create_product(
            &NewProduct::try_new(1, "Coffee", "RUB")
                .expect("valid product")
                .with_sku(ProductSku::new("COF-1").expect("sku"))
                .with_description(ProductDescription::new("<p>Arabica</p>").expect("description")),
        )
        .expect("create product");

    repo.replace_product_price_levels(
        product.id,
        hub_id,
        &[NewProductPriceLevelRate::new(
            product.id,
            retail.id,
            PriceCents::new(1250).expect("price"),
        )],
    )
    .expect("attach price levels");

    let fetched = repo
        .get_product_by_id(product.id, hub_id)
        .expect("fetch product")
        .expect("product should exist");

    fetched.id.get()
}

fn api_scope() -> actix_web::Scope {
    web::scope("/api")
        .service(api_v1_vendors)
        .service(api_v1_vendor)
        .service(api_v1_create_vendor)
        .service(api_v1_update_vendor)
        .service(api_v1_local_users)
        .service(api_v1_create_local_user)
        .service(api_v1_assign_vendor_user)
        .service(api_v1_clear_vendor_user)
        .service(api_v1_orders)
        .service(api_v1_order)
        .service(api_v1_update_order)
        .service(api_v1_update_order_product_approvals)
        .service(api_v1_products)
        .service(api_v1_product)
        .service(api_v1_create_product)
        .service(api_v1_update_product)
        .service(api_v1_upload_products)
}

fn seed_vendor(repo: &DieselRepository) -> i32 {
    let vendor = repo
        .create_vendor(&NewVendor::new(
            HubId::new(1).expect("valid hub id"),
            VendorName::new("Vendor One").expect("valid vendor name"),
        ))
        .expect("create vendor");

    vendor.id.get()
}

fn seed_local_user(repo: &DieselRepository) -> i32 {
    let user = repo
        .create_user(&NewUser::new(
            HubId::new(1).expect("valid hub id"),
            UserName::new("Vendor User").expect("valid user name"),
            UserEmail::new("vendor-user@example.com").expect("valid user email"),
        ))
        .expect("create local user");

    user.id.get()
}

#[actix_web::test]
async fn api_orders_collection_returns_typed_resource_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let order_id = seed_order(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let orders_request = test::TestRequest::get()
        .uri("/api/v1/orders")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let orders_response = test::call_service(&app, orders_request).await;
    assert_eq!(orders_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(orders_response).await;
    assert_eq!(payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        payload["items"][0]["id"].as_i64(),
        Some(i64::from(order_id))
    );
    assert_eq!(payload["items"][0]["reference"].as_str(), Some("ORD-101"));
    assert_eq!(payload["items"][0]["status"].as_str(), Some("Pending"));
    assert_eq!(payload["items"][0]["products_count"].as_u64(), Some(1));
    assert_eq!(payload["pagination"]["page"].as_u64(), Some(1));
    assert_eq!(payload["pagination"]["total_items"].as_u64(), Some(1));
    assert_eq!(
        payload["active_filters"]["search"].as_str(),
        Option::<&str>::None
    );
}

#[actix_web::test]
async fn api_order_details_returns_typed_resource_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let order_id = seed_order(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let details_request = test::TestRequest::get()
        .uri(&format!("/api/v1/orders/{order_id}"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let details_response = test::call_service(&app, details_request).await;
    assert_eq!(details_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(details_response).await;
    assert_eq!(payload["id"].as_i64(), Some(i64::from(order_id)));
    assert_eq!(payload["reference"].as_str(), Some("ORD-101"));
    assert_eq!(payload["status"].as_str(), Some("Pending"));
    assert_eq!(
        payload["crm_service_url"].as_str(),
        Some("https://crm.example.com")
    );
    assert_eq!(payload["customer"]["name"].as_str(), Some("John Doe"));
    assert_eq!(payload["customer"]["phone"].as_str(), Some("+12125550123"));
    assert_eq!(payload["products"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["products"][0]["product_id"].as_i64(), Some(8));
    assert_eq!(payload["products"][0]["sku"].as_str(), Some("COF-1"));
    assert_eq!(
        payload["products"][0]["approved_quantity"].as_i64(),
        Some(2)
    );
    assert_eq!(
        payload["products"][0]["default_price_cents"].as_i64(),
        Some(6000)
    );
}

#[actix_web::test]
async fn api_orders_collection_rejects_authenticated_users_without_orders_role() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    seed_order(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "admin@example.com",
            "name": "Admin User",
            "roles": ["orders_admin"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let orders_request = test::TestRequest::get()
        .uri("/api/v1/orders")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let orders_response = test::call_service(&app, orders_request).await;
    assert_eq!(orders_response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn api_order_details_returns_not_found_for_missing_order() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let details_request = test::TestRequest::get()
        .uri("/api/v1/orders/9999")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let details_response = test::call_service(&app, details_request).await;
    assert_eq!(details_response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn api_vendors_collection_returns_typed_resource_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let vendor_id = seed_vendor(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "admin@example.com",
            "name": "Admin User",
            "roles": ["orders_admin"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let vendors_request = test::TestRequest::get()
        .uri("/api/v1/vendors")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let vendors_response = test::call_service(&app, vendors_request).await;
    assert_eq!(vendors_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(vendors_response).await;
    assert_eq!(payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        payload["items"][0]["id"].as_i64(),
        Some(i64::from(vendor_id))
    );
    assert_eq!(payload["items"][0]["name"].as_str(), Some("Vendor One"));
    assert_eq!(payload["pagination"]["page"].as_u64(), Some(1));
}

#[actix_web::test]
async fn api_vendor_user_mutations_return_json_payloads() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let vendor_id = seed_vendor(&repo);
    let user_id = seed_local_user(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "admin@example.com",
            "name": "Admin User",
            "roles": ["orders_admin"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let assign_request = test::TestRequest::post()
        .uri("/api/v1/vendors/assignments")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .set_json(json!({
            "user_id": user_id,
            "vendor_id": vendor_id,
        }))
        .to_request();
    let assign_response = test::call_service(&app, assign_request).await;
    assert_eq!(assign_response.status(), StatusCode::OK);
    let assign_payload: Value = test::read_body_json(assign_response).await;
    assert_eq!(
        assign_payload["message"].as_str(),
        Some("Пользователь привязан к поставщику.")
    );

    let users_request = test::TestRequest::get()
        .uri("/api/v1/users")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let users_response = test::call_service(&app, users_request).await;
    assert_eq!(users_response.status(), StatusCode::OK);
    let users_payload: Value = test::read_body_json(users_response).await;
    assert_eq!(users_payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        users_payload["items"][0]["vendor_id"].as_i64(),
        Some(i64::from(vendor_id))
    );

    let clear_request = test::TestRequest::delete()
        .uri(&format!("/api/v1/vendors/assignments/{user_id}"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let clear_response = test::call_service(&app, clear_request).await;
    assert_eq!(clear_response.status(), StatusCode::OK);
    let clear_payload: Value = test::read_body_json(clear_response).await;
    assert_eq!(
        clear_payload["message"].as_str(),
        Some("Привязка пользователя удалена.")
    );
}

#[actix_web::test]
async fn api_update_order_returns_updated_order_details_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let order_id = seed_order(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let update_request = test::TestRequest::put()
        .uri(&format!("/api/v1/orders/{order_id}"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .set_json(json!({
            "order_id": order_id,
            "status": "Processing",
            "reference": "ORD-500",
            "notes": "Комментарий",
            "shipping_address": "Москва",
            "consignee": "Иван",
            "delivery_notes": "Позвонить заранее",
            "payer": "ООО Ромашка",
        }))
        .to_request();
    let update_response = test::call_service(&app, update_request).await;
    assert_eq!(update_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(update_response).await;
    assert_eq!(payload["message"].as_str(), Some("Заказ обновлён."));
    assert_eq!(payload["order"]["status"].as_str(), Some("Processing"));
    assert_eq!(payload["order"]["reference"].as_str(), Some("ORD-500"));
    assert_eq!(payload["order"]["notes"].as_str(), Some("Комментарий"));
    assert_eq!(
        payload["order"]["shipping_address"].as_str(),
        Some("Москва")
    );
    assert_eq!(payload["order"]["consignee"].as_str(), Some("Иван"));
    assert_eq!(
        payload["order"]["delivery_notes"].as_str(),
        Some("Позвонить заранее")
    );
    assert_eq!(payload["order"]["payer"].as_str(), Some("ООО Ромашка"));
}

#[actix_web::test]
async fn api_update_order_returns_typed_validation_errors() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let order_id = seed_order(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let update_request = test::TestRequest::put()
        .uri(&format!("/api/v1/orders/{order_id}"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .set_json(json!({
            "order_id": order_id,
            "status": "",
            "reference": null,
            "notes": null,
            "shipping_address": null,
            "consignee": null,
            "delivery_notes": null,
            "payer": null,
        }))
        .to_request();
    let update_response = test::call_service(&app, update_request).await;
    assert_eq!(update_response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let payload: Value = test::read_body_json(update_response).await;
    assert_eq!(payload["field_errors"][0]["field"].as_str(), Some("status"));
    assert_eq!(
        payload["field_errors"][0]["message"].as_str(),
        Some("Выберите статус заказа.")
    );
}

#[actix_web::test]
async fn api_update_order_product_approvals_returns_refreshed_order_details() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let order_id = seed_order(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let update_request = test::TestRequest::put()
        .uri(&format!("/api/v1/orders/{order_id}/products/approvals"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .set_json(json!({
            "approvals": [
                {
                    "product_id": 8,
                    "approved_quantity": 1,
                }
            ],
        }))
        .to_request();
    let update_response = test::call_service(&app, update_request).await;
    assert_eq!(update_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(update_response).await;
    assert_eq!(payload["message"].as_str(), Some("Количество обновлено."));
    assert_eq!(
        payload["order"]["products"][0]["approved_quantity"].as_i64(),
        Some(1)
    );
    assert_eq!(payload["order"]["total_cents"].as_i64(), Some(2500));
}

#[actix_web::test]
async fn api_products_collection_returns_typed_resource_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let product_id = seed_product(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let products_request = test::TestRequest::get()
        .uri("/api/v1/products")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let products_response = test::call_service(&app, products_request).await;
    assert_eq!(products_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(products_response).await;
    assert_eq!(payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        payload["items"][0]["id"].as_i64(),
        Some(i64::from(product_id))
    );
    assert_eq!(payload["items"][0]["name"].as_str(), Some("Coffee"));
    assert_eq!(payload["items"][0]["sku"].as_str(), Some("COF-1"));
    assert_eq!(
        payload["items"][0]["description_html"].as_str(),
        Some("<p>Arabica</p>")
    );
    assert_eq!(payload["items"][0]["currency"].as_str(), Some("RUB"));
    assert_eq!(
        payload["items"][0]["price_levels"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(payload["pagination"]["page"].as_u64(), Some(1));
    assert_eq!(payload["pagination"]["total_items"].as_u64(), Some(1));
    assert_eq!(
        payload["editor_options"]["price_levels"][0]["name"].as_str(),
        Some("Retail")
    );
    assert_eq!(
        payload["active_filters"]["show_archived"].as_bool(),
        Some(false)
    );
}

#[actix_web::test]
async fn api_product_details_returns_typed_resource_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let product_id = seed_product(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let product_request = test::TestRequest::get()
        .uri(&format!("/api/v1/products/{product_id}"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .to_request();
    let product_response = test::call_service(&app, product_request).await;
    assert_eq!(product_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(product_response).await;
    assert_eq!(payload["id"].as_i64(), Some(i64::from(product_id)));
    assert_eq!(payload["name"].as_str(), Some("Coffee"));
    assert_eq!(payload["sku"].as_str(), Some("COF-1"));
    assert_eq!(payload["currency"].as_str(), Some("RUB"));
    assert_eq!(payload["description_html"].as_str(), Some("<p>Arabica</p>"));
    assert_eq!(payload["price_levels"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        payload["editor_options"]["price_levels"][0]["name"].as_str(),
        Some("Retail")
    );
}

#[actix_web::test]
async fn api_create_product_returns_created_product_payload() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    repo.create_price_level(&NewPriceLevel::try_new(1, "Retail", false).expect("price level"))
        .expect("create price level");
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let create_request = test::TestRequest::post()
        .uri("/api/v1/products")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .set_json(json!({
            "name": "Tea",
            "sku": "TEA-1",
            "description": "<p>Green tea</p>",
            "units": "шт",
            "currency": "RUB",
            "tag_ids": [],
            "image_urls": "https://example.com/tea.png",
            "price_levels": [],
            "amount": 1.0
        }))
        .to_request();
    let create_response = test::call_service(&app, create_request).await;
    assert_eq!(create_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(create_response).await;
    assert_eq!(payload["message"].as_str(), Some("Товар добавлен."));
    assert_eq!(payload["product"]["name"].as_str(), Some("Tea"));
    assert_eq!(payload["product"]["sku"].as_str(), Some("TEA-1"));
    assert_eq!(
        payload["product"]["description_html"].as_str(),
        Some("<p>Green tea</p>")
    );
    assert_eq!(
        payload["product"]["image_urls"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(payload["product"]["amount"].as_str(), Some("1"));
}

#[actix_web::test]
async fn api_update_product_returns_typed_validation_errors() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let product_id = seed_product(&repo);
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let update_request = test::TestRequest::put()
        .uri(&format!("/api/v1/products/{product_id}"))
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .set_json(json!({
            "product_id": product_id,
            "name": "",
            "sku": "COF-1",
            "description": "<p>Arabica</p>",
            "units": "шт",
            "currency": "RUB",
            "image_urls": "",
            "is_archived": false,
            "tag_ids": [],
            "price_levels": [],
            "amount": 1.0
        }))
        .to_request();
    let update_response = test::call_service(&app, update_request).await;
    assert_eq!(update_response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let payload: Value = test::read_body_json(update_response).await;
    assert_eq!(
        payload["message"].as_str(),
        Some("Ошибка валидации формы: Название товара обязательно.")
    );
    assert_eq!(payload["field_errors"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["field_errors"][0]["field"].as_str(), Some("name"));
    assert_eq!(
        payload["field_errors"][0]["message"].as_str(),
        Some("Название товара обязательно.")
    );
}

#[actix_web::test]
async fn api_upload_products_returns_created_count() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    repo.create_price_level(&NewPriceLevel::try_new(1, "Retail", false).expect("price level"))
        .expect("create price level");
    let common_config = common_config();
    let secret_key = Key::from(common_config.secret.as_bytes());

    let app = test::init_service(
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key)
                    .cookie_secure(false)
                    .build(),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config))
            .app_data(web::Data::new(app_config()))
            .service(test_login)
            .service(api_scope()),
    )
    .await;

    let login_request = test::TestRequest::post()
        .uri("/test/login")
        .set_json(json!({
            "hub_id": 1,
            "email": "orders@example.com",
            "name": "Orders User",
            "roles": ["orders"],
        }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);

    let boundary = "X-BOUNDARY";
    let csv = "name,currency\nMilk,RUB\n";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"csv\"; filename=\"products.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
    );

    let upload_request = test::TestRequest::post()
        .uri("/api/v1/products/upload")
        .insert_header((header::COOKIE, cookie_header_value(&login_response)))
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(multipart_body)
        .to_request();
    let upload_response = test::call_service(&app, upload_request).await;
    assert_eq!(upload_response.status(), StatusCode::OK);

    let payload: Value = test::read_body_json(upload_response).await;
    assert_eq!(payload["message"].as_str(), Some("Загружено товаров: 1."));
    assert_eq!(payload["created_count"].as_u64(), Some(1));
}
