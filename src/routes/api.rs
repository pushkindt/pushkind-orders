use actix_web::{HttpResponse, Responder, get, put, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::dto::main::IndexQuery;
use crate::forms::price_levels::AssignClientPriceLevelForm;
use crate::repository::DieselRepository;
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
    match main_service::load_index_page(params.0, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response.orders),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list orders: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/client-price-levels")]
/// Return a JSON list of client price level assignments.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_client_price_levels(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match load_client_price_level_assignments(&user, repo.get_ref()) {
        Ok(assignments) => HttpResponse::Ok().json(assignments),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load client price levels: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/v1/client-price-levels")]
/// Assign a price level to a client.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_update_client_price_level(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    payload: web::Json<AssignClientPriceLevelForm>,
) -> impl Responder {
    let payload = payload.into_inner();
    let log_phone = payload.phone.clone();

    match assign_price_level_to_client(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({"error": message}))
        }
        Err(err) => {
            log::error!("Failed to assign price level to client {log_phone}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
