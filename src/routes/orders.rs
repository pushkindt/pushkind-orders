use actix_web::{HttpRequest, HttpResponse, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;

use crate::frontend::{
    FRONTEND_ORDER_DOCUMENT, FrontendAssetError, frontend_document_path, open_frontend_html,
};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, orders as order_service};

#[get("/order/{order_id}")]
/// Render the order details page for a specific order.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a redirect to `/na`.
pub async fn show_order(
    request: HttpRequest,
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> HttpResponse {
    let order_id = path.into_inner();

    match order_service::ensure_order_page_access(order_id, &user, repo.get_ref()) {
        Ok(()) => match open_frontend_html(frontend_document_path(FRONTEND_ORDER_DOCUMENT)).await {
            Ok(file) => file.into_response(&request),
            Err(FrontendAssetError::Read(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                HttpResponse::ServiceUnavailable().body(
                    "Orders frontend assets are not built yet. Run `cd frontend && npm run build`.",
                )
            }
            Err(error) => {
                log::error!("Failed to open orders order frontend document: {error}");
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => redirect("/"),
        Err(err) => {
            log::error!("Failed to load order {order_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
