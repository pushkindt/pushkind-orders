use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::repository::DieselRepository;
use crate::services::{ServiceError, order as order_service};

#[get("/order/{order_id}")]
pub async fn show_order(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let order_id = path.into_inner();

    match order_service::load_order_details(repo.get_ref(), &user, order_id) {
        Ok(details) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("order", &details.order);
            context.insert("customer", &details.customer);
            render_template(&tera, "order/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Заказ не найден или уже удалён.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to load order {order_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
