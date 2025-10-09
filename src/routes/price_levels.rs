use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::price_levels::{PriceLevelsQuery, load_price_levels};

#[get("/price-levels")]
pub async fn show_price_levels(
    params: web::Query<PriceLevelsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_price_levels(repo.get_ref(), &user, params.0) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "price_levels",
                &server_config.auth_service_url,
            );
            context.insert("price_levels", &data.price_levels);
            context.insert("search", &data.search);
            context.insert("search_action", &"/price-levels");
            render_template(&tera, "price_levels/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list price levels: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
