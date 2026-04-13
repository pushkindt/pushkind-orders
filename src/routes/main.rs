use actix_web::{HttpRequest, HttpResponse, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;

use crate::frontend::{
    FRONTEND_INDEX_DOCUMENT, FrontendAssetError, frontend_document_path, open_frontend_html,
};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, resolve_hub_access};

#[get("/")]
/// Serve the React-backed orders dashboard shell after a lightweight access check.
pub async fn show_index(
    request: HttpRequest,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> HttpResponse {
    match resolve_hub_access(&user, repo.get_ref()) {
        Ok(_) => match open_frontend_html(frontend_document_path(FRONTEND_INDEX_DOCUMENT)).await {
            Ok(file) => file.into_response(&request),
            Err(FrontendAssetError::Read(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                HttpResponse::ServiceUnavailable().body(
                    "Orders frontend assets are not built yet. Run `cd frontend && npm run build`.",
                )
            }
            Err(error) => {
                log::error!("Failed to open orders index frontend document: {error}");
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(error) => {
            log::error!("Failed to authorize access to the orders dashboard: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
