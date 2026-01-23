use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::types::{UserEmail, UserId, UserName, VendorId, VendorName};

/// Result type returned by the vendor form helpers.
pub type VendorFormResult<T> = Result<T, VendorFormError>;

/// Errors that can occur while processing vendor forms.
#[derive(Debug, Error)]
pub enum VendorFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The provided vendor id is invalid.
    #[error("vendor id is invalid")]
    InvalidVendorId,
    /// The provided user id is invalid.
    #[error("user id is invalid")]
    InvalidUserId,
    /// The provided name is invalid.
    #[error("vendor name is invalid")]
    InvalidVendorName,
    /// The provided user name is invalid.
    #[error("user name is invalid")]
    InvalidUserName,
    /// The provided user email is invalid.
    #[error("user email is invalid")]
    InvalidUserEmail,
}

/// Form payload emitted when submitting the "Add vendor" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddVendorForm {
    /// Name entered by the user.
    #[validate(length(min = 1))]
    pub name: String,
}

/// Normalized payload for creating a vendor.
#[derive(Debug, Clone)]
pub struct AddVendorPayload {
    pub name: VendorName,
}

impl TryFrom<AddVendorForm> for AddVendorPayload {
    type Error = VendorFormError;

    fn try_from(value: AddVendorForm) -> Result<Self, Self::Error> {
        value.validate()?;

        VendorName::new(value.name)
            .map(|name| Self { name })
            .map_err(|_| VendorFormError::InvalidVendorName)
    }
}

/// Form payload emitted when editing an existing vendor.
#[derive(Debug, Deserialize, Validate)]
pub struct EditVendorForm {
    /// Identifier of the vendor to update.
    #[validate(range(min = 1))]
    pub vendor_id: i32,
    /// Updated name supplied by the user.
    #[validate(length(min = 1))]
    pub name: String,
}

/// Normalized payload for updating a vendor.
#[derive(Debug, Clone)]
pub struct EditVendorPayload {
    pub vendor_id: VendorId,
    pub name: VendorName,
}

impl TryFrom<EditVendorForm> for EditVendorPayload {
    type Error = VendorFormError;

    fn try_from(value: EditVendorForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let vendor_id =
            VendorId::new(value.vendor_id).map_err(|_| VendorFormError::InvalidVendorId)?;
        let name = VendorName::new(value.name).map_err(|_| VendorFormError::InvalidVendorName)?;

        Ok(Self { vendor_id, name })
    }
}

/// Form payload emitted when assigning a user to a vendor.
#[derive(Debug, Deserialize, Validate)]
pub struct AssignVendorUserForm {
    /// Identifier of the user to assign.
    #[validate(range(min = 1))]
    pub user_id: i32,
    /// Identifier of the vendor.
    #[validate(range(min = 1))]
    pub vendor_id: i32,
}

/// Normalized payload for assigning a vendor user.
#[derive(Debug, Clone)]
pub struct AssignVendorUserPayload {
    pub user_id: UserId,
    pub vendor_id: VendorId,
}

impl TryFrom<AssignVendorUserForm> for AssignVendorUserPayload {
    type Error = VendorFormError;

    fn try_from(value: AssignVendorUserForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let user_id = UserId::new(value.user_id).map_err(|_| VendorFormError::InvalidUserId)?;
        let vendor_id =
            VendorId::new(value.vendor_id).map_err(|_| VendorFormError::InvalidVendorId)?;

        Ok(Self { user_id, vendor_id })
    }
}

/// Form payload emitted when clearing a vendor assignment.
#[derive(Debug, Deserialize, Validate)]
pub struct ClearVendorUserForm {
    /// Identifier of the user to clear.
    #[validate(range(min = 1))]
    pub user_id: i32,
}

/// Normalized payload for clearing a vendor assignment.
#[derive(Debug, Clone)]
pub struct ClearVendorUserPayload {
    pub user_id: UserId,
}

impl TryFrom<ClearVendorUserForm> for ClearVendorUserPayload {
    type Error = VendorFormError;

    fn try_from(value: ClearVendorUserForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let user_id = UserId::new(value.user_id).map_err(|_| VendorFormError::InvalidUserId)?;

        Ok(Self { user_id })
    }
}

/// Form payload emitted when submitting the "Add user" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddUserForm {
    /// Name entered by the user.
    #[validate(length(min = 1))]
    pub name: String,
    /// Email entered by the user.
    #[validate(email)]
    pub email: String,
}

/// Normalized payload for creating a user.
#[derive(Debug, Clone)]
pub struct AddUserPayload {
    pub name: UserName,
    pub email: UserEmail,
}

impl TryFrom<AddUserForm> for AddUserPayload {
    type Error = VendorFormError;

    fn try_from(value: AddUserForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let name = UserName::new(value.name).map_err(|_| VendorFormError::InvalidUserName)?;
        let email = UserEmail::new(value.email).map_err(|_| VendorFormError::InvalidUserEmail)?;
        Ok(Self { name, email })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_vendor_form_converts() {
        let form = AddVendorForm {
            name: "  Vendor A  ".to_string(),
        };

        let payload: AddVendorPayload = form.try_into().expect("conversion");
        assert_eq!(payload.name.as_str(), "Vendor A");
    }

    #[test]
    fn edit_vendor_form_converts() {
        let form = EditVendorForm {
            vendor_id: 5,
            name: "  Vendor B  ".to_string(),
        };

        let payload: EditVendorPayload = form.try_into().expect("conversion");
        assert_eq!(payload.vendor_id.get(), 5);
        assert_eq!(payload.name.as_str(), "Vendor B");
    }

    #[test]
    fn assign_vendor_user_form_converts() {
        let form = AssignVendorUserForm {
            user_id: 3,
            vendor_id: 9,
        };

        let payload: AssignVendorUserPayload = form.try_into().expect("conversion");
        assert_eq!(payload.user_id.get(), 3);
        assert_eq!(payload.vendor_id.get(), 9);
    }

    #[test]
    fn clear_vendor_user_form_converts() {
        let form = ClearVendorUserForm { user_id: 3 };

        let payload: ClearVendorUserPayload = form.try_into().expect("conversion");
        assert_eq!(payload.user_id.get(), 3);
    }
}
