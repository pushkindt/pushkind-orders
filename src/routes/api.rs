use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::repository::DieselRepository;
use crate::services::main::IndexQuery;
use crate::services::{ServiceError, main as main_service};

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
