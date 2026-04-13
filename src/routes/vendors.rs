use actix_web::{HttpRequest, HttpResponse, Responder, get};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;

use crate::frontend::{
    FRONTEND_VENDORS_DOCUMENT, FrontendAssetError, frontend_document_path, open_frontend_html,
};
use crate::services::ServiceError;
use crate::services::vendors::ensure_vendors_page_access;

#[get("/vendors")]
/// Render the vendors management page with a React-owned shell.
pub async fn show_vendors(request: HttpRequest, user: AuthenticatedUser) -> impl Responder {
    match ensure_vendors_page_access(&user) {
        Ok(()) => match open_frontend_html(frontend_document_path(FRONTEND_VENDORS_DOCUMENT)).await
        {
            Ok(file) => file.into_response(&request),
            Err(FrontendAssetError::Read(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                HttpResponse::ServiceUnavailable().body(
                    "Orders frontend assets are not built yet. Run `cd frontend && npm run build`.",
                )
            }
            Err(error) => {
                log::error!("Failed to open orders vendors frontend document: {error}");
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(err) => {
            log::error!("Failed to authorize access to the vendors page: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
