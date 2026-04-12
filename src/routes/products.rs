use actix_web::{HttpRequest, HttpResponse, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;

use crate::frontend::{
    FRONTEND_PRODUCTS_DOCUMENT, FrontendAssetError, frontend_document_path, open_frontend_html,
};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, products};

#[get("/products")]
/// Render the products management page with search, filters, and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a redirect to `/na`.
pub async fn show_products(
    request: HttpRequest,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> HttpResponse {
    match products::ensure_products_page_access(&user, repo.get_ref()) {
        Ok(()) => {
            match open_frontend_html(frontend_document_path(FRONTEND_PRODUCTS_DOCUMENT)).await {
                Ok(file) => file.into_response(&request),
                Err(FrontendAssetError::Read(error))
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    HttpResponse::ServiceUnavailable().body(
                        "Orders frontend assets are not built yet. Run `cd frontend && npm run build`.",
                    )
                }
                Err(error) => {
                    log::error!("Failed to open orders products frontend document: {error}");
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(err) => {
            log::error!("Failed to authorize access to the products page: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
