//! Pushkind orders service library providing HTTP server setup and application wiring.

use actix_cors::Cors;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, dev::Server, middleware, web};
use pushkind_common::db::establish_connection_pool;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::logout;
use std::net::TcpListener;

use crate::models::config::{AppConfig, Settings};
use crate::repository::DieselRepository;
use crate::routes::api::{
    api_v1_assign_vendor_user, api_v1_categories, api_v1_category, api_v1_clear_vendor_user,
    api_v1_client_price_levels, api_v1_create_category, api_v1_create_local_user,
    api_v1_create_price_level, api_v1_create_product, api_v1_create_tag, api_v1_create_vendor,
    api_v1_delete_category, api_v1_delete_price_level, api_v1_delete_tag, api_v1_delete_vendor,
    api_v1_iam, api_v1_local_users, api_v1_no_access, api_v1_order, api_v1_orders,
    api_v1_price_level, api_v1_price_levels, api_v1_product, api_v1_products, api_v1_tag,
    api_v1_tags, api_v1_update_category, api_v1_update_client_price_level, api_v1_update_order,
    api_v1_update_order_product_approvals, api_v1_update_price_level, api_v1_update_product,
    api_v1_update_tag, api_v1_update_vendor, api_v1_upload_products, api_v1_vendor, api_v1_vendors,
};
use crate::routes::aux::not_assigned;
use crate::routes::categories::show_categories;
use crate::routes::main::show_index;
use crate::routes::orders::show_order;
use crate::routes::price_levels::show_price_levels;
use crate::routes::products::show_products;
use crate::routes::store::{
    create_store_order_handler, get_store_product, list_store_categories,
    list_store_orders_handler, list_store_products, list_store_tags, list_store_vendors,
    update_store_order_handler,
};
use crate::routes::tags::show_tags;
use crate::routes::vendors::show_vendors;

pub mod domain;
pub mod dto;
pub mod error_conversions;
pub mod forms;
pub mod frontend;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod services;

pub const SERVICE_ACCESS_ROLE: &str = "orders";
pub const ADMIN_ACCESS_ROLE: &str = "orders_admin";
pub const VENDOR_ACCESS_ROLE: &str = "orders_vendor";

/// Builds and runs the Actix-Web HTTP server using the provided configuration.
pub async fn run(settings: Settings) -> std::io::Result<()> {
    let bind_address = (settings.server.address.clone(), settings.server.port);
    let listener = TcpListener::bind(bind_address)?;

    build_server(listener, settings.app)?.await
}

/// Builds an Actix-Web HTTP server on a pre-bound listener.
pub fn build_server(listener: TcpListener, app_config: AppConfig) -> std::io::Result<Server> {
    let common_config = CommonServerConfig {
        auth_service_url: app_config.auth_service_url.to_string(),
        secret: app_config.secret.clone(),
    };

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&app_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity and sessions.
    let secret_key = Key::from(app_config.secret.as_bytes());

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", app_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(not_assigned)
            .service(
                web::scope("/api/v1/store")
                    .service(list_store_products)
                    .service(get_store_product)
                    .service(list_store_categories)
                    .service(list_store_orders_handler)
                    .service(update_store_order_handler)
                    .service(list_store_tags)
                    .service(list_store_vendors)
                    .service(create_store_order_handler),
            )
            .service(
                web::scope("/api")
                    .service(api_v1_iam)
                    .service(api_v1_no_access)
                    .service(api_v1_categories)
                    .service(api_v1_category)
                    .service(api_v1_create_category)
                    .service(api_v1_update_category)
                    .service(api_v1_delete_category)
                    .service(api_v1_tags)
                    .service(api_v1_tag)
                    .service(api_v1_create_tag)
                    .service(api_v1_update_tag)
                    .service(api_v1_delete_tag)
                    .service(api_v1_price_levels)
                    .service(api_v1_price_level)
                    .service(api_v1_create_price_level)
                    .service(api_v1_update_price_level)
                    .service(api_v1_delete_price_level)
                    .service(api_v1_vendors)
                    .service(api_v1_vendor)
                    .service(api_v1_create_vendor)
                    .service(api_v1_update_vendor)
                    .service(api_v1_delete_vendor)
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
                    .service(api_v1_client_price_levels)
                    .service(api_v1_update_client_price_level),
            )
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(show_index)
                    .service(show_order)
                    .service(show_categories)
                    .service(show_tags)
                    .service(show_price_levels)
                    .service(show_products)
                    .service(show_vendors)
                    .service(logout),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(app_config.clone()))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
