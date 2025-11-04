use actix_web::{App, http::StatusCode, test, web};
use pushkind_orders::domain::{category::NewCategory, product::NewProduct, tag::NewTag};
use pushkind_orders::repository::{CategoryWriter, DieselRepository, ProductWriter, TagWriter};
use pushkind_orders::routes::store::{list_store_categories, list_store_products, list_store_tags};
use pushkind_orders::services::store::{StoreCategory, StoreProduct, StoreTag};

mod common;

#[actix_web::test]
async fn store_endpoints_return_data() {
    let test_db = common::TestDb::new("routes_store_endpoints_return_data.db");
    let repo = DieselRepository::new(test_db.pool());

    let category = repo
        .create_category(&NewCategory::new(1, "Beverages"))
        .expect("create category");
    let tag = repo
        .create_tag(&NewTag::new(1, "Organic"))
        .expect("create tag");

    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD").with_category_id(category.id))
        .expect("create product");
    repo.replace_product_tags(product.id, 1, &[tag.id])
        .expect("attach tag");

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
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
async fn store_routes_reject_invalid_hub_id() {
    let test_db = common::TestDb::new("routes_store_invalid_hub_id.db");
    let repo = DieselRepository::new(test_db.pool());

    let app_repo = repo.clone();
    let app = test::init_service(
        App::new().app_data(web::Data::new(app_repo)).service(
            web::scope("/api/v1/store")
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
