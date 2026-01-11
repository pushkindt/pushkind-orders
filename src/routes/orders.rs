use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{base_context, redirect, render_template};
use serde_json::json;
use tera::{Context, Tera};

use crate::dto::orders::OrderProductApprovalPayload;
use crate::forms::orders::EditOrderForm;
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, orders as order_service};

#[get("/order/{order_id}")]
/// Render the order details page for a specific order.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn show_order(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
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
            context.insert("crm_service_url", &server_config.crm_service_url);
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

#[post("/orders/{order_id}/edit")]
/// Accept edits submitted from the order details modal.
pub async fn edit_order(
    path: web::Path<i32>,
    form: web::Form<EditOrderForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let order_id = path.into_inner();
    let order_path = format!("/order/{order_id}");

    let form = form.into_inner();
    if form.order_id != order_id {
        log::warn!(
            "Order id mismatch when editing order: path={order_id} payload={}",
            form.order_id
        );
        FlashMessage::error("Некорректные данные формы.").send();
        return redirect(order_path.as_str());
    }

    match order_service::update_order(repo.get_ref(), &user, order_id, form) {
        Ok(_) => {
            FlashMessage::success("Заказ обновлён.").send();
            redirect(order_path.as_str())
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(order_path.as_str())
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Заказ не найден или уже удалён.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to update order {order_id}: {err}");
            FlashMessage::error("Не удалось обновить заказ.").send();
            redirect(order_path.as_str())
        }
    }
}

#[get("/order/{order_id}/modal")]
/// Render the edit order modal for a specific order.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn show_edit_order_modal(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let order_id = path.into_inner();

    match order_service::load_order_details(repo.get_ref(), &user, order_id) {
        Ok(details) => {
            let mut context = Context::new();
            context.insert("order", &details.order);
            render_template(&tera, "order/edit_order_modal.html", &context)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load order {order_id} modal: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/orders/{order_id}/products/approvals")]
/// Update approved quantities for order products and return refreshed order details.
pub async fn update_order_product_approvals_handler(
    path: web::Path<i32>,
    payload: web::Json<Vec<OrderProductApprovalPayload>>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let order_id = path.into_inner();

    match order_service::update_order_product_approvals(
        repo.get_ref(),
        &user,
        order_id,
        payload.into_inner(),
    ) {
        Ok(details) => HttpResponse::Ok().json(details),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to update order products for {order_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
