//! Auxiliary routes for React-owned frontend documents.

use actix_web::{HttpRequest, HttpResponse, get};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::frontend::{
    FRONTEND_NO_ACCESS_DOCUMENT, FrontendAssetError, frontend_document_path, open_frontend_html,
};

#[get("/na")]
pub async fn not_assigned(request: HttpRequest, _user: AuthenticatedUser) -> HttpResponse {
    match open_frontend_html(frontend_document_path(FRONTEND_NO_ACCESS_DOCUMENT)).await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable().body(
                "Orders frontend assets are not built yet. Run `cd frontend && npm run build`.",
            )
        }
        Err(error) => {
            log::error!("Failed to open orders no-access frontend document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
