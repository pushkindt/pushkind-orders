use actix_web::{HttpResponse, Responder, get, put, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::forms::price_levels::AssignClientPriceLevelPayload;
use crate::repository::DieselRepository;
use crate::services::main::IndexQuery;
use crate::services::price_levels::{
    assign_price_level_to_client, load_client_price_level_assignments,
};
use crate::services::{ServiceError, main as main_service};
use serde_json::json;

#[get("/v1/orders")]
/// Return a JSON list of orders with optional search and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_orders(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match main_service::load_index_page(repo.get_ref(), &user, params.0) {
        Ok(response) => HttpResponse::Ok().json(response.orders),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list orders: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/client-price-levels")]
pub async fn api_v1_client_price_levels(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match load_client_price_level_assignments(repo.get_ref(), &user) {
        Ok(assignments) => HttpResponse::Ok().json(assignments),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load client price levels: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/v1/client-price-levels")]
pub async fn api_v1_update_client_price_level(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    payload: web::Json<AssignClientPriceLevelPayload>,
) -> impl Responder {
    let payload = payload.into_inner();
    let log_email = payload.email.clone();
    let log_phone = payload.phone.clone();

    match assign_price_level_to_client(repo.get_ref(), &user, payload) {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({"error": message}))
        }
        Err(err) => {
            log::error!(
                "Failed to assign price level to client {log_email} / {:?}: {err}",
                log_phone
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}
