use actix_session::{Session, SessionGetError, SessionInsertError};
use thiserror::Error;

use crate::domain::customer::Customer;

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
