use actix_web::{HttpRequest, HttpResponse, Responder, get};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;

use crate::frontend::{
    FRONTEND_TAGS_DOCUMENT, FrontendAssetError, frontend_document_path, open_frontend_html,
};
use crate::services::ServiceError;
use crate::services::tags::ensure_tags_page_access;

#[get("/tags")]
/// Render the tags management page with search and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a redirect to `/na`.
pub async fn show_tags(request: HttpRequest, user: AuthenticatedUser) -> impl Responder {
    match ensure_tags_page_access(&user) {
        Ok(()) => match open_frontend_html(frontend_document_path(FRONTEND_TAGS_DOCUMENT)).await {
            Ok(file) => file.into_response(&request),
            Err(FrontendAssetError::Read(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                HttpResponse::ServiceUnavailable().body(
                    "Orders frontend assets are not built yet. Run `cd frontend && npm run build`.",
                )
            }
            Err(error) => {
                log::error!("Failed to open orders tags frontend document: {error}");
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(err) => {
            log::error!("Failed to authorize access to the tags page: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
