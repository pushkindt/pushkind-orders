use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::{
    domain::price_level::{NewPriceLevel, UpdatePriceLevel},
    domain::types::{HubId, PriceLevelName, TypeConstraintError, normalize_phone_to_e164},
    forms::sanitize_text,
};

/// Maximum length allowed for a price level name.
const NAME_MAX_LEN: usize = 128;
const NAME_MAX_LEN_VALIDATOR: u64 = NAME_MAX_LEN as u64;
const PHONE_MAX_LEN: usize = 64;
const PHONE_MAX_LEN_VALIDATOR: u64 = PHONE_MAX_LEN as u64;

/// Result type returned by the price level form helpers.
pub type PriceLevelFormResult<T> = Result<T, PriceLevelFormError>;

/// Errors that can occur while processing price level forms.
#[derive(Debug, Error)]
pub enum PriceLevelFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The provided name is empty after sanitization.
    #[error("price level name cannot be empty")]
    EmptyName,
    /// The provided email is empty after sanitization.
    #[error("email is incorrect")]
    IncorrectEmail,
    /// The provided phone is missing.
    #[error("phone number is required")]
    MissingPhone,
    /// The provided phone fails validation.
    #[error("phone number is incorrect")]
    IncorrectPhone,
}

/// Form payload emitted when submitting the "Add price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddPriceLevelForm {
    /// Name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Is this a default price level?
    #[serde(default)]
    pub default: bool,
}

/// Payload emitted when assigning a price level to a client.
#[derive(Debug, Deserialize, Validate)]
pub struct AssignClientPriceLevelPayload {
    /// Customer name used when creating missing records.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Customer email used as part of the composite key.
    #[serde(default)]
    #[validate(email)]
    pub email: Option<String>,
    /// Customer phone used as part of the composite key.
    #[validate(length(min = 1, max = PHONE_MAX_LEN_VALIDATOR))]
    pub phone: String,
    /// Selected price level identifier. `None` restores the default hub level.
    #[validate(range(min = 1))]
    #[serde(default)]
    pub price_level_id: Option<i32>,
}

impl AssignClientPriceLevelPayload {
    /// Validates and normalizes the payload into an assignment request.
    pub fn into_assignment_request(self) -> PriceLevelFormResult<AssignClientPriceLevelInput> {
        self.validate()?;

        let sanitized_name = match sanitize_text(&self.name) {
            Some(name) => name,
            None => return Err(PriceLevelFormError::EmptyName),
        };

        let normalized_email = self.email.as_ref().and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_lowercase())
            }
        });

        let normalized_phone = normalize_phone_to_e164(&self.phone).map_err(|err| match err {
            TypeConstraintError::EmptyString => PriceLevelFormError::MissingPhone,
            TypeConstraintError::InvalidPhone => PriceLevelFormError::IncorrectPhone,
            _ => PriceLevelFormError::IncorrectPhone,
        })?;

        Ok(AssignClientPriceLevelInput {
            name: sanitized_name,
            email: normalized_email,
            phone: normalized_phone,
            price_level_id: self.price_level_id,
        })
    }
}

/// Normalized payload that can be passed to the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignClientPriceLevelInput {
    pub name: String,
    pub email: Option<String>,
    pub phone: String,
    pub price_level_id: Option<i32>,
}

impl AddPriceLevelForm {
    /// Validates and sanitizes the payload into a domain `NewPriceLevel`.
    pub fn into_new_price_level(self, hub_id: i32) -> PriceLevelFormResult<NewPriceLevel> {
        self.validate()?;

        let hub_id = HubId::new(hub_id)
            .map_err(|_| PriceLevelFormError::Validation(ValidationErrors::new()))?;

        let name = sanitize_text(&self.name).ok_or(PriceLevelFormError::EmptyName)?;
        let name = PriceLevelName::new(name).map_err(|_| PriceLevelFormError::EmptyName)?;

        Ok(NewPriceLevel::new(hub_id, name, self.default))
    }
}

/// Form payload emitted when submitting the "Edit price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct EditPriceLevelForm {
    /// Updated name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Updated default flag for the price level.
    #[serde(default)]
    pub default: bool,
}

impl EditPriceLevelForm {
    /// Validates and sanitizes the payload into a domain `UpdatePriceLevel`.
    pub fn into_update_price_level(self) -> PriceLevelFormResult<UpdatePriceLevel> {
        self.validate()?;

        let name = sanitize_text(&self.name).ok_or(PriceLevelFormError::EmptyName)?;
        let name = PriceLevelName::new(name).map_err(|_| PriceLevelFormError::EmptyName)?;

        Ok(UpdatePriceLevel::new(name, self.default))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_price_level_form_sanitizes_and_converts() {
        let form = AddPriceLevelForm {
            name: "  Premium\tLevel  ".to_string(),
            default: false,
        };

        let new_level = form.into_new_price_level(5).expect("expected success");

        assert_eq!(new_level.hub_id.get(), 5);
        assert_eq!(new_level.name.as_str(), "Premium\tLevel");
    }

    #[test]
    fn assign_client_price_level_payload_validates_positive_ids() {
        let payload = AssignClientPriceLevelPayload {
            name: "   User Name  ".to_string(),
            email: Some("USER@example.com".to_string()),
            phone: "  +1999  ".to_string(),
            price_level_id: Some(3),
        };

        let assignment = payload
            .into_assignment_request()
            .expect("expected valid payload");

        assert_eq!(assignment.name, "User Name");
        assert_eq!(assignment.email.as_deref(), Some("user@example.com"));
        assert_eq!(assignment.phone, "+1999");
        assert_eq!(assignment.price_level_id, Some(3));
    }

    #[test]
    fn assign_client_price_level_payload_rejects_invalid_ids() {
        let payload = AssignClientPriceLevelPayload {
            name: "".to_string(),
            email: Some("".to_string()),
            phone: "+1888".to_string(),
            price_level_id: Some(0),
        };

        let result = payload.into_assignment_request();

        assert!(result.is_err(), "expected validation error");
    }

    #[test]
    fn assign_client_price_level_payload_requires_phone() {
        let payload = AssignClientPriceLevelPayload {
            name: "User".to_string(),
            email: None,
            phone: "   ".to_string(),
            price_level_id: None,
        };

        let result = payload.into_assignment_request();

        assert!(matches!(result, Err(PriceLevelFormError::MissingPhone)));
    }

    #[test]
    fn add_price_level_form_rejects_empty() {
        let form = AddPriceLevelForm {
            name: "   ".to_string(),
            default: false,
        };

        let result = form.into_new_price_level(1);

        assert!(matches!(result, Err(PriceLevelFormError::EmptyName)));
    }

    #[test]
    fn edit_price_level_form_sanitizes_and_converts() {
        let form = EditPriceLevelForm {
            name: "  Updated\nName  ".to_string(),
            default: true,
        };

        let update = form.into_update_price_level().expect("expected success");

        assert_eq!(update.name.as_str(), "Updated\nName");
        assert!(update.is_default);
    }

    #[test]
    fn edit_price_level_form_rejects_empty() {
        let form = EditPriceLevelForm {
            name: " \t".to_string(),
            default: false,
        };

        let result = form.into_update_price_level();

        assert!(matches!(result, Err(PriceLevelFormError::EmptyName)));
    }
}
