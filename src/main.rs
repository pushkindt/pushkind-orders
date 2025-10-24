use std::env;

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
use tera::Tera;

use pushkind_orders::repository::DieselRepository;
use pushkind_orders::routes::categories::{
    add_category, assign_category, delete_category, edit_category, show_categories,
};
use pushkind_orders::routes::main::show_index;
use pushkind_orders::routes::price_levels::{
    add_price_level, delete_price_level, show_price_levels, upload_price_levels,
};
use pushkind_orders::routes::products::{add_product, show_products, upload_products};
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
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(show_index)
                    .service(show_categories)
                    .service(add_category)
                    .service(assign_category)
                    .service(edit_category)
                    .service(delete_category)
                    .service(show_tags)
                    .service(add_tag)
                    .service(edit_tag)
                    .service(delete_tag)
                    .service(show_price_levels)
                    .service(add_price_level)
                    .service(upload_price_levels)
                    .service(delete_price_level)
                    .service(show_products)
                    .service(add_product)
                    .service(upload_products)
                    .service(logout),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config.clone()))
    })
    .bind((address, port))?
    .run()
    .await
}
