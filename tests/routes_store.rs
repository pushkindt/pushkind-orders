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

    let _category = repo
        .create_category(&NewCategory::new(1, "Beverages"))
        .expect("create category");
    let tag = repo
        .create_tag(&NewTag::new(1, "Organic"))
        .expect("create tag");

    let product = repo
        .create_product(&NewProduct::new(1, "Coffee", "USD"))
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
            "/api/v1/store/1/products?category_id={}",
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
