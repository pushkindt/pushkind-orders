//! Service layer orchestrating domain logic and repository operations.

pub mod categories;
pub mod main;
pub mod orders;
pub mod price_levels;
pub mod products;
pub mod store;
pub mod tags;

pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

use crate::domain::types::TypeConstraintError;

impl From<TypeConstraintError> for ServiceError {
    fn from(_: TypeConstraintError) -> Self {
        ServiceError::Internal
    }
}
