//! Pushkind orders service library providing HTTP server setup and application wiring.

use actix_cors::Cors;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
use pushkind_common::db::establish_connection_pool;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{logout, not_assigned};
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use tera::Tera;

use crate::models::config::{ServerConfig, ZmqSenders};
use crate::repository::DieselRepository;
use crate::routes::api::{
    api_v1_client_price_levels, api_v1_orders, api_v1_update_client_price_level,
};
use crate::routes::categories::{
    add_category, delete_category, edit_category, show_categories, show_edit_category_modal,
};
use crate::routes::main::show_index;
use crate::routes::orders::{
    edit_order, show_edit_order_modal, show_order, update_order_product_approvals_handler,
};
use crate::routes::price_levels::{
    add_price_level, delete_price_level, edit_price_level, show_edit_price_level_modal,
    show_price_levels,
};
use crate::routes::products::{add_product, edit_product, show_products, upload_products};
use crate::routes::store::{
    create_store_order_handler, get_store_product, list_store_categories,
    list_store_orders_handler, list_store_products, list_store_tags, request_store_auth_otp,
    update_store_order_handler, verify_store_auth_otp,
};
use crate::routes::store_session::get_store_session;
use crate::routes::tags::{add_tag, delete_tag, edit_tag, show_edit_tag_modal, show_tags};

pub mod domain;
pub mod dto;
pub mod error_conversions;
pub mod forms;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod services;

pub const SERVICE_ACCESS_ROLE: &str = "orders";
pub const VENDOR_ACCESS_ROLE: &str = "orders_vendor";

/// Builds and runs the Actix-Web HTTP server using the provided configuration.
pub async fn run(server_config: ServerConfig) -> std::io::Result<()> {
    let common_config = CommonServerConfig {
        auth_service_url: server_config.auth_service_url.to_string(),
        secret: server_config.secret.clone(),
    };

    // Start background ZeroMQ publishers used for outbound notifications.
    let sms_sender = ZmqSender::start(ZmqSenderOptions::pub_default(&server_config.zmq_sms_pub))
        .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ SMS sender: {e}")))?;
    let clients_sender = ZmqSender::start(ZmqSenderOptions::pub_default(
        &server_config.zmq_clients_pub,
    ))
    .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ clients sender: {e}")))?;

    let zmq_senders = web::Data::new(ZmqSenders {
        sms: sms_sender,
        clients: clients_sender,
    });

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&server_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity, sessions, and flash messages.
    let secret_key = Key::from(server_config.secret.as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = Tera::new(&server_config.templates_dir)
        .map_err(|e| std::io::Error::other(format!("Template parsing error(s): {e}")))?;

    let bind_address = (server_config.address.clone(), server_config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", server_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(not_assigned)
            .service(
                web::scope("/api/v1/store")
                    .wrap(
                        SessionMiddleware::builder(
                            CookieSessionStore::default(),
                            secret_key.clone(),
                        )
                        .cookie_name("store-session".to_string())
                        .cookie_secure(false)
                        .cookie_domain(Some(format!(".{}", server_config.domain)))
                        .build(),
                    )
                    .service(list_store_products)
                    .service(get_store_product)
                    .service(list_store_categories)
                    .service(list_store_orders_handler)
                    .service(update_store_order_handler)
                    .service(list_store_tags)
                    .service(request_store_auth_otp)
                    .service(verify_store_auth_otp)
                    .service(create_store_order_handler)
                    .service(get_store_session),
            )
            .service(
                web::scope("/api")
                    .wrap(RedirectUnauthorized)
                    .service(api_v1_orders)
                    .service(api_v1_client_price_levels)
                    .service(api_v1_update_client_price_level),
            )
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(show_index)
                    .service(show_order)
                    .service(show_categories)
                    .service(add_category)
                    .service(edit_category)
                    .service(show_edit_category_modal)
                    .service(delete_category)
                    .service(show_tags)
                    .service(add_tag)
                    .service(edit_tag)
                    .service(show_edit_tag_modal)
                    .service(delete_tag)
                    .service(show_price_levels)
                    .service(add_price_level)
                    .service(edit_price_level)
                    .service(show_edit_price_level_modal)
                    .service(delete_price_level)
                    .service(show_products)
                    .service(add_product)
                    .service(edit_product)
                    .service(upload_products)
                    .service(update_order_product_approvals_handler)
                    .service(edit_order)
                    .service(show_edit_order_modal)
                    .service(logout),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(zmq_senders.clone())
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(server_config.clone()))
    })
    .bind(bind_address)?
    .run()
    .await
}
