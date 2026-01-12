use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::{
    domain::{
        price_level::{NewPriceLevel, UpdatePriceLevel},
        types::{
            CategoryId, CustomerName, HubId, PhoneNumber, PriceLevelId, PriceLevelName, PublicId,
        },
    },
    forms::sanitize_text,
};

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
    InvalidPriceLevelName,
    /// The provided public id is empty after sanitization.
    #[error("public id is incorrect")]
    IncorrectPublicId,
    /// The provided customer name is empty after sanitization.
    #[error("customer name cannot be empty")]
    InvalidCustomerName,
    /// The provided phone fails validation.
    #[error("phone number is incorrect")]
    IncorrectPhone,
    /// Incorrect price level id provided.
    #[error("price level id must be a positive integer")]
    InvalidPriceLevelId,
    /// Incorrect base price level id provided.
    #[error("base price level id must be a positive integer")]
    InvalidBasePriceLevelId,
    /// Price modifier is out of supported range.
    #[error("price modifier is outside of the allowed range")]
    InvalidPriceModifier,
    /// Excluded categories list must not be empty when not using all categories.
    #[error("at least one excluded category must be selected")]
    ExcludedCategoriesRequired,
    /// Excluded category ids must be positive.
    #[error("excluded category id must be a positive integer")]
    InvalidExcludedCategoryId,
}

/// Modifier type for price level adjustments.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PriceModifierKind {
    Percent,
    Fixed,
}

/// Form payload emitted when submitting the "Add price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddPriceLevelForm {
    /// Name entered by the user.
    #[validate(length(min = 1))]
    pub name: String,
    /// Is this a default price level?
    #[serde(default)]
    pub default: bool,
    /// Base price level used for adjustments.
    #[validate(range(min = 1))]
    pub base_price_level_id: i32,
    /// Price modifier value.
    pub price_modifier: i32,
    /// Determines if modifier is percentage or fixed amount.
    pub price_modifier_kind: PriceModifierKind,
    /// Apply to all categories.
    #[serde(default)]
    pub use_all_categories: bool,
    /// Category ids excluded from modifier.
    #[serde(default)]
    pub excluded_category_ids: Vec<i32>,
}

/// Payload emitted when assigning a price level to a client.
#[derive(Debug, Deserialize, Validate)]
pub struct AssignClientPriceLevelForm {
    /// Customer name used when creating missing records.
    #[validate(length(min = 1))]
    pub name: String,
    /// Customer phone used as part of the composite key.
    #[validate(length(min = 1))]
    pub phone: String,
    /// Customer public id used as part of the composite key.
    #[validate(length(min = 1))]
    pub public_id: String,
    /// Selected price level identifier. `None` restores the default hub level.
    #[validate(range(min = 1))]
    #[serde(default)]
    pub price_level_id: Option<i32>,
}

/// Normalized payload that can be passed to the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignClientPriceLevelPayload {
    pub name: CustomerName,
    pub public_id: PublicId,
    pub phone: PhoneNumber,
    pub price_level_id: Option<PriceLevelId>,
}

impl TryFrom<AssignClientPriceLevelForm> for AssignClientPriceLevelPayload {
    type Error = PriceLevelFormError;

    fn try_from(value: AssignClientPriceLevelForm) -> Result<Self, Self::Error> {
        value.validate().map_err(PriceLevelFormError::Validation)?;

        Ok(Self {
            name: CustomerName::new(value.name)
                .map_err(|_| PriceLevelFormError::InvalidCustomerName)?,
            public_id: PublicId::new(value.public_id)
                .map_err(|_| PriceLevelFormError::IncorrectPublicId)?,
            phone: PhoneNumber::new(value.phone)
                .map_err(|_| PriceLevelFormError::IncorrectPhone)?,
            price_level_id: match value.price_level_id {
                Some(id) => Some(
                    PriceLevelId::new(id).map_err(|_| PriceLevelFormError::InvalidPriceLevelId)?,
                ),
                None => None,
            },
        })
    }
}

impl AddPriceLevelForm {
    /// Validates and sanitizes the payload into a domain `NewPriceLevel`.
    pub fn into_new_price_level(self, hub_id: i32) -> PriceLevelFormResult<NewPriceLevel> {
        self.validate()?;

        let hub_id = HubId::new(hub_id)
            .map_err(|_| PriceLevelFormError::Validation(ValidationErrors::new()))?;

        let name = sanitize_text(&self.name).ok_or(PriceLevelFormError::InvalidPriceLevelName)?;
        let name =
            PriceLevelName::new(name).map_err(|_| PriceLevelFormError::InvalidPriceLevelName)?;

        let _base_price_level_id = PriceLevelId::new(self.base_price_level_id)
            .map_err(|_| PriceLevelFormError::InvalidBasePriceLevelId)?;

        let _price_modifier = match self.price_modifier_kind {
            PriceModifierKind::Percent => {
                if (-100..=100).contains(&self.price_modifier) {
                    self.price_modifier
                } else {
                    return Err(PriceLevelFormError::InvalidPriceModifier);
                }
            }
            PriceModifierKind::Fixed => {
                if (-1_000_000..=1_000_000).contains(&self.price_modifier) {
                    self.price_modifier
                } else {
                    return Err(PriceLevelFormError::InvalidPriceModifier);
                }
            }
        };

        let excluded_category_ids: Vec<CategoryId> = self
            .excluded_category_ids
            .into_iter()
            .map(CategoryId::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PriceLevelFormError::InvalidExcludedCategoryId)?;

        if !self.use_all_categories && excluded_category_ids.is_empty() {
            return Err(PriceLevelFormError::ExcludedCategoriesRequired);
        }

        Ok(NewPriceLevel::new(hub_id, name, self.default))
    }
}

/// Form payload emitted when submitting the "Edit price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct EditPriceLevelForm {
    /// Updated name entered by the user.
    #[validate(length(min = 1))]
    pub name: String,
    /// Updated default flag for the price level.
    #[serde(default)]
    pub default: bool,
}

impl EditPriceLevelForm {
    /// Validates and sanitizes the payload into a domain `UpdatePriceLevel`.
    pub fn into_update_price_level(self) -> PriceLevelFormResult<UpdatePriceLevel> {
        self.validate()?;

        let name = sanitize_text(&self.name).ok_or(PriceLevelFormError::InvalidPriceLevelName)?;
        let name =
            PriceLevelName::new(name).map_err(|_| PriceLevelFormError::InvalidPriceLevelName)?;

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
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            use_all_categories: true,
            excluded_category_ids: Vec::new(),
        };

        let new_level = form.into_new_price_level(5).expect("expected success");

        assert_eq!(new_level.hub_id.get(), 5);
        assert_eq!(new_level.name.as_str(), "Premium\tLevel");
    }

    #[test]
    fn assign_client_price_level_payload_validates_positive_ids() {
        let payload = AssignClientPriceLevelForm {
            name: "   User Name  ".to_string(),
            phone: "  +1999  ".to_string(),
            price_level_id: Some(3),
            public_id: "123123".to_string(),
        };

        let assignment: AssignClientPriceLevelPayload =
            payload.try_into().expect("expected valid payload");

        assert_eq!(assignment.name.as_str(), "User Name");
        assert_eq!(assignment.phone.as_str(), "+1999");
        assert_eq!(assignment.price_level_id.map(|id| id.get()), Some(3));
        assert_eq!(assignment.public_id.as_str(), "123123");
    }

    #[test]
    fn assign_client_price_level_payload_rejects_invalid_ids() {
        let payload = AssignClientPriceLevelForm {
            name: "".to_string(),
            phone: "+1888".to_string(),
            price_level_id: Some(0),
            public_id: "123123".to_string(),
        };

        let result: PriceLevelFormResult<AssignClientPriceLevelPayload> = payload.try_into();

        assert!(result.is_err(), "expected validation error");
    }

    #[test]
    fn assign_client_price_level_payload_requires_phone() {
        let payload = AssignClientPriceLevelForm {
            name: "User".to_string(),
            phone: "   ".to_string(),
            price_level_id: None,
            public_id: "123123".to_string(),
        };

        let result: Result<AssignClientPriceLevelPayload, PriceLevelFormError> = payload.try_into();

        assert!(matches!(result, Err(PriceLevelFormError::IncorrectPhone)));
    }

    #[test]
    fn add_price_level_form_rejects_empty() {
        let form = AddPriceLevelForm {
            name: "   ".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            use_all_categories: true,
            excluded_category_ids: Vec::new(),
        };

        let result = form.into_new_price_level(1);

        assert!(matches!(
            result,
            Err(PriceLevelFormError::InvalidPriceLevelName)
        ));
    }

    #[test]
    fn add_price_level_form_requires_excluded_categories_when_disabled() {
        let form = AddPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 5,
            price_modifier_kind: PriceModifierKind::Percent,
            use_all_categories: false,
            excluded_category_ids: Vec::new(),
        };

        let result = form.into_new_price_level(1);

        assert!(matches!(
            result,
            Err(PriceLevelFormError::ExcludedCategoriesRequired)
        ));
    }

    #[test]
    fn add_price_level_form_rejects_out_of_range_modifier() {
        let form = AddPriceLevelForm {
            name: "Wholesale".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 500,
            price_modifier_kind: PriceModifierKind::Percent,
            use_all_categories: true,
            excluded_category_ids: Vec::new(),
        };

        let result = form.into_new_price_level(1);

        assert!(matches!(
            result,
            Err(PriceLevelFormError::InvalidPriceModifier)
        ));
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

        assert!(matches!(
            result,
            Err(PriceLevelFormError::InvalidPriceLevelName)
        ));
    }
}
