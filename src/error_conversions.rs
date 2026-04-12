//! Error conversion glue for `data` feature consumers.
//!
//! The domain layer must not depend on service/repository error types, but
//! downstream crates using `pushkind-orders` data types may
//! still want convenient conversions.

use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::services::errors::ServiceError;

use crate::domain::types::TypeConstraintError;
use crate::forms::categories::CategoryFormError;
use crate::forms::orders::{EditOrderFormError, UpdateOrderApprovalsFormError};
use crate::forms::price_levels::PriceLevelFormError;
use crate::forms::products::ProductFormError;
use crate::forms::store::{StoreFormError, StoreOrderUpdateError};
use crate::forms::tags::TagFormError;
use crate::forms::vendors::VendorFormError;

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

impl From<PriceLevelFormError> for ServiceError {
    fn from(val: PriceLevelFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<CategoryFormError> for ServiceError {
    fn from(val: CategoryFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<EditOrderFormError> for ServiceError {
    fn from(val: EditOrderFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<UpdateOrderApprovalsFormError> for ServiceError {
    fn from(val: UpdateOrderApprovalsFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<ProductFormError> for ServiceError {
    fn from(val: ProductFormError) -> Self {
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

impl From<TagFormError> for ServiceError {
    fn from(val: TagFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<VendorFormError> for ServiceError {
    fn from(val: VendorFormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}
