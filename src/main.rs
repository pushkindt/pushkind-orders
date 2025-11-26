use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
use dotenvy::dotenv;
use pushkind_common::db::establish_connection_pool;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{logout, not_assigned};
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use pushkind_orders::models::config::ServerConfig;
use tera::Tera;

use pushkind_orders::repository::DieselRepository;
use pushkind_orders::routes::api::{
    api_v1_client_price_levels, api_v1_orders, api_v1_update_client_price_level,
};
use pushkind_orders::routes::categories::{
    add_category, delete_category, edit_category, show_categories,
};
use pushkind_orders::routes::main::show_index;
use pushkind_orders::routes::order::show_order;
use pushkind_orders::routes::price_levels::{
    add_price_level, delete_price_level, edit_price_level, show_price_levels,
};
use pushkind_orders::routes::products::{
    add_product, edit_product, show_products, upload_products,
};
use pushkind_orders::routes::store::{
    create_store_order_handler, get_store_product, list_store_categories,
    list_store_orders_handler, list_store_products, list_store_tags, request_store_auth_otp,
    verify_store_auth_otp,
};
use pushkind_orders::routes::store_session::get_store_session;
use pushkind_orders::routes::tags::{add_tag, delete_tag, edit_tag, show_tags};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or("app.db".to_string());
    let port = env::var("PORT").unwrap_or("8080".to_string());
    let port = port.parse::<u16>().unwrap_or(8080);
    let address = env::var("ADDRESS").unwrap_or("127.0.0.1".to_string());

    let secret = env::var("SECRET_KEY");
    let secret_key = match &secret {
        Ok(key) => Key::from(key.as_bytes()),
        Err(_) => Key::generate(),
    };

    let auth_service_url = env::var("AUTH_SERVICE_URL");
    let auth_service_url = match auth_service_url {
        Ok(auth_service_url) => auth_service_url,
        Err(_) => {
            log::error!("AUTH_SERVICE_URL environment variable not set");
            std::process::exit(1);
        }
    };

    let common_config = CommonServerConfig {
        secret: secret.unwrap_or_default(),
        auth_service_url,
    };

    let crm_service_url = env::var("CRM_SERVICE_URL").unwrap_or_default();
    let sms_sender = env::var("SMS_SENDER").unwrap_or("cns.shared".to_string());
    let server_config = ServerConfig {
        crm_service_url,
        sms_sender,
    };

    let zmq_address = env::var("ZMQ_SMS_PUB").unwrap_or("tcp://127.0.0.1:5561".to_string());
    let zmq_sender = match ZmqSender::start(ZmqSenderOptions::pub_default(&zmq_address)) {
        Ok(zmq_sender) => zmq_sender,
        Err(e) => {
            log::error!("Failed to start ZMQ sender: {e}");
            std::process::exit(1);
        }
    };
    let zmq_sender = Arc::new(zmq_sender);

    let domain = env::var("DOMAIN").unwrap_or("localhost".to_string());

    let pool = match establish_connection_pool(&database_url) {
        Ok(pool) => pool,
        Err(e) => {
            log::error!("Failed to establish database connection: {e}");
            std::process::exit(1);
        }
    };
    let repo = DieselRepository::new(pool);

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            log::error!("Parsing error(s): {e}");
            std::process::exit(1);
        }
    };

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{domain}")))
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
                        .cookie_domain(Some(format!(".{domain}")))
                        .build(),
                    )
                    .service(list_store_products)
                    .service(get_store_product)
                    .service(list_store_categories)
                    .service(list_store_orders_handler)
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
                    .service(delete_category)
                    .service(show_tags)
                    .service(add_tag)
                    .service(edit_tag)
                    .service(delete_tag)
                    .service(show_price_levels)
                    .service(add_price_level)
                    .service(edit_price_level)
                    .service(delete_price_level)
                    .service(show_products)
                    .service(add_product)
                    .service(edit_product)
                    .service(upload_products)
                    .service(logout),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(server_config.clone()))
    })
    .bind((address, port))?
    .run()
    .await
}
