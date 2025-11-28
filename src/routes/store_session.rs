use actix_session::{Session, SessionGetError, SessionInsertError};
use actix_web::{HttpResponse, Responder, get, web};
use log::error;
use serde::Deserialize;
use thiserror::Error;

use crate::domain::customer::Customer;
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::store::load_store_session_customer;

const STORE_SESSION_CUSTOMER_KEY: &str = "store_customer";

/// Errors produced while reading or writing the store session state.
#[derive(Debug, Error)]
pub enum StoreSessionError {
    /// Serialization of the customer failed.
    #[error("failed to store customer session: {0}")]
    Insert(#[from] SessionInsertError),
    /// Deserialization of the customer failed.
    #[error("failed to read customer session: {0}")]
    Get(#[from] SessionGetError),
}

/// Persist the authenticated customer inside the store session.
pub fn set_store_customer(session: &Session, customer: &Customer) -> Result<(), StoreSessionError> {
    session.insert(STORE_SESSION_CUSTOMER_KEY, customer)?;
    Ok(())
}

/// Retrieve the authenticated customer from the store session, if present.
pub fn get_store_customer(session: &Session) -> Result<Option<Customer>, StoreSessionError> {
    Ok(session.get(STORE_SESSION_CUSTOMER_KEY)?)
}

/// Remove the authenticated customer from the store session.
pub fn clear_store_customer(session: &Session) -> Result<(), StoreSessionError> {
    session.remove(STORE_SESSION_CUSTOMER_KEY);
    Ok(())
}

/// Read the authenticated customer only if they belong to `hub_id`.
pub fn get_store_customer_for_hub(
    session: &Session,
    hub_id: i32,
) -> Result<Option<Customer>, StoreSessionError> {
    match get_store_customer(session)? {
        Some(customer) if customer.hub_id.get() == hub_id => Ok(Some(customer)),
        Some(_) => {
            clear_store_customer(session)?;
            Ok(None)
        }
        None => Ok(None),
    }
}

#[derive(Debug, Deserialize)]
struct HubPath {
    hub_id: String,
}

#[get("/{hub_id}/auth/session")]
/// Validate and return the authenticated customer from the store session.
///
/// Returns `401 Unauthorized` if no valid session exists for the specified hub.
pub async fn get_store_session(
    path: web::Path<HubPath>,
    repo: web::Data<DieselRepository>,
    session: Session,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let session_customer = match get_store_customer_for_hub(&session, hub_id) {
        Ok(Some(customer)) => customer,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to read store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match load_store_session_customer(repo.get_ref(), &session_customer) {
        Ok(customer) => HttpResponse::Ok().json(customer),
        Err(ServiceError::Unauthorized) => {
            if let Err(err) = clear_store_customer(&session) {
                error!("Failed to clear store session cookie for hub {hub_id}: {err}");
            }
            HttpResponse::Unauthorized().finish()
        }
        Err(err) => {
            error!("Failed to validate store session customer for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
