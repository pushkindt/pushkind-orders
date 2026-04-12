use std::borrow::Cow;

use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::types::{UserEmail, UserId, UserName, VendorId, VendorName};

/// Result type returned by the vendor form helpers.
pub type VendorFormResult<T> = Result<T, VendorFormError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

/// Errors that can occur while processing vendor forms.
#[derive(Debug, Error)]
pub enum VendorFormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),
    #[error("Идентификатор поставщика указан неверно.")]
    InvalidVendorId,
    #[error("Идентификатор пользователя указан неверно.")]
    InvalidUserId,
    #[error("Название поставщика указано неверно.")]
    InvalidVendorName,
    #[error("Имя пользователя указано неверно.")]
    InvalidUserName,
    #[error("Электронный адрес пользователя указан неверно.")]
    InvalidUserEmail,
}

impl VendorFormError {
    pub fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            Self::InvalidVendorId => vec![field_error("vendor_id", self.to_string())],
            Self::InvalidUserId => vec![field_error("user_id", self.to_string())],
            Self::InvalidVendorName => vec![field_error("name", self.to_string())],
            Self::InvalidUserName => vec![field_error("name", self.to_string())],
            Self::InvalidUserEmail => vec![field_error("email", self.to_string())],
        }
    }
}

/// Form payload emitted when submitting the "Add vendor" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddVendorForm {
    /// Name entered by the user.
    #[validate(length(min = 1, message = "Название поставщика обязательно."))]
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
    #[validate(range(min = 1, message = "Идентификатор поставщика указан неверно."))]
    pub vendor_id: i32,
    /// Updated name supplied by the user.
    #[validate(length(min = 1, message = "Название поставщика обязательно."))]
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
    #[validate(range(min = 1, message = "Идентификатор пользователя указан неверно."))]
    pub user_id: i32,
    /// Identifier of the vendor.
    #[validate(range(min = 1, message = "Идентификатор поставщика указан неверно."))]
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
    #[validate(range(min = 1, message = "Идентификатор пользователя указан неверно."))]
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
    #[validate(length(min = 1, message = "Имя пользователя обязательно."))]
    pub name: String,
    /// Email entered by the user.
    #[validate(email(message = "Электронный адрес пользователя указан неверно."))]
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

fn field_error(field: impl Into<Cow<'static, str>>, message: impl Into<String>) -> FormFieldError {
    FormFieldError {
        field: field.into(),
        message: Cow::Owned(message.into()),
    }
}

fn collect_validation_errors(errors: &ValidationErrors) -> Vec<FormFieldError> {
    let mut field_errors = Vec::new();

    for (field, errors) in errors.field_errors() {
        for error in errors {
            field_errors.push(field_error(
                match field.as_ref() {
                    "vendor_id" => Cow::Borrowed("vendor_id"),
                    "user_id" => Cow::Borrowed("user_id"),
                    "name" => Cow::Borrowed("name"),
                    "email" => Cow::Borrowed("email"),
                    other => Cow::Owned(other.to_string()),
                },
                error
                    .message
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "Некорректное значение.".to_string()),
            ));
        }
    }

    field_errors.sort_by(|left, right| left.field.cmp(&right.field));
    field_errors
}

fn validation_errors_display(errors: &ValidationErrors) -> String {
    let collected = collect_validation_errors(errors);

    if collected.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else if collected.len() == 1 {
        format!("Ошибка валидации формы: {}", collected[0].message)
    } else {
        format!(
            "Ошибка валидации формы: {}",
            collected
                .into_iter()
                .map(|error| error.message.into_owned())
                .collect::<Vec<_>>()
                .join("; ")
        )
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
