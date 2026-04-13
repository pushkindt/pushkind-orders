//! Error conversion glue for `data` feature consumers.
//!
//! The domain layer must not depend on service/repository error types, but
//! downstream crates using `pushkind-orders` data types may
//! still want convenient conversions.

use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::services::errors::ServiceError;

use crate::domain::types::TypeConstraintError;
use crate::forms::FormError;
use crate::forms::store::{StoreFormError, StoreOrderUpdateError};

impl From<TypeConstraintError> for ServiceError {
    fn from(val: TypeConstraintError) -> Self {
        ServiceError::TypeConstraint(val.to_string())
    }
}

impl From<TypeConstraintError> for RepositoryError {
    fn from(val: TypeConstraintError) -> Self {
        RepositoryError::ValidationError(val.to_string())
    }
}

impl From<FormError> for ServiceError {
    fn from(val: FormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<StoreFormError> for ServiceError {
    fn from(val: StoreFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<StoreOrderUpdateError> for ServiceError {
    fn from(val: StoreOrderUpdateError) -> Self {
        ServiceError::Form(val.to_string())
    }
}
