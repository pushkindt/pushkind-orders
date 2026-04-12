use std::borrow::Cow;

use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::types::{
    CategoryId, CustomerName, PhoneNumber, PriceLevelId, PriceLevelName, ProductId, PublicId,
};

/// Result type returned by the price level form helpers.
pub type PriceLevelFormResult<T> = Result<T, PriceLevelFormError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

/// Errors that can occur while processing price level forms.
#[derive(Debug, Error)]
pub enum PriceLevelFormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),
    #[error("Название уровня цен указано неверно.")]
    InvalidPriceLevelName,
    #[error("Публичный идентификатор клиента указан неверно.")]
    IncorrectPublicId,
    #[error("Имя клиента указано неверно.")]
    InvalidCustomerName,
    #[error("Телефон клиента указан неверно.")]
    IncorrectPhone,
    #[error("Идентификатор уровня цен указан неверно.")]
    InvalidPriceLevelId,
    #[error("Базовый уровень цен указан неверно.")]
    InvalidBasePriceLevelId,
    #[error("Модификатор цены указан неверно.")]
    InvalidPriceModifier,
    #[error("Категория в исключениях указана неверно.")]
    InvalidExcludedCategoryId,
    #[error("Товар в исключениях указан неверно.")]
    InvalidExcludedProductId,
    #[error("Товар во включениях указан неверно.")]
    InvalidIncludedProductId,
}

impl PriceLevelFormError {
    pub fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            Self::InvalidPriceLevelName => vec![field_error("name", self.to_string())],
            Self::IncorrectPublicId => vec![field_error("public_id", self.to_string())],
            Self::InvalidCustomerName => vec![field_error("name", self.to_string())],
            Self::IncorrectPhone => vec![field_error("phone", self.to_string())],
            Self::InvalidPriceLevelId => vec![field_error("price_level_id", self.to_string())],
            Self::InvalidBasePriceLevelId => {
                vec![field_error("base_price_level_id", self.to_string())]
            }
            Self::InvalidPriceModifier => vec![field_error("price_modifier", self.to_string())],
            Self::InvalidExcludedCategoryId => {
                vec![field_error("excluded_category_ids", self.to_string())]
            }
            Self::InvalidExcludedProductId => {
                vec![field_error("excluded_product_ids", self.to_string())]
            }
            Self::InvalidIncludedProductId => {
                vec![field_error("included_product_ids", self.to_string())]
            }
        }
    }
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
    #[validate(length(min = 1, message = "Название уровня цен обязательно."))]
    pub name: String,
    /// Is this a default price level?
    #[serde(default)]
    pub default: bool,
    /// Base price level used for adjustments.
    #[validate(range(min = 1, message = "Выберите базовый уровень цен."))]
    pub base_price_level_id: i32,
    /// Price modifier value.
    pub price_modifier: i32,
    /// Determines if modifier is percentage or fixed amount.
    pub price_modifier_kind: PriceModifierKind,
    /// Category ids excluded from modifier.
    #[serde(default)]
    pub excluded_category_ids: Vec<i32>,
    /// Product ids excluded from modifier.
    #[serde(default)]
    pub excluded_product_ids: Vec<i32>,
    /// Product ids explicitly included regardless of exclusions.
    #[serde(default)]
    pub included_product_ids: Vec<i32>,
}

/// Normalized payload for creating a price level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPriceLevelPayload {
    pub name: PriceLevelName,
    pub default: bool,
    pub modifier_input: PriceLevelModifierInput,
}

/// Normalized payload describing how to apply a price modifier to products.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceLevelModifierInput {
    pub base_price_level_id: PriceLevelId,
    pub price_modifier: i32,
    pub price_modifier_kind: PriceModifierKind,
    pub excluded_category_ids: Vec<CategoryId>,
    pub excluded_product_ids: Vec<ProductId>,
    pub included_product_ids: Vec<ProductId>,
}

/// Payload emitted when assigning a price level to a client.
#[derive(Debug, Deserialize, Validate)]
pub struct AssignClientPriceLevelForm {
    /// Customer name used when creating missing records.
    #[validate(length(min = 1, message = "Имя клиента обязательно."))]
    pub name: String,
    /// Customer phone used as part of the composite key.
    #[validate(length(min = 1, message = "Телефон клиента обязателен."))]
    pub phone: String,
    /// Customer public id used as part of the composite key.
    #[validate(length(min = 1, message = "Публичный идентификатор клиента обязателен."))]
    pub public_id: String,
    /// Selected price level identifier. `None` restores the default hub level.
    #[validate(range(min = 1, message = "Уровень цен указан неверно."))]
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
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

impl TryFrom<AddPriceLevelForm> for AddPriceLevelPayload {
    type Error = PriceLevelFormError;

    fn try_from(value: AddPriceLevelForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let name = PriceLevelName::new(value.name)
            .map_err(|_| PriceLevelFormError::InvalidPriceLevelName)?;

        let base_price_level_id = PriceLevelId::new(value.base_price_level_id)
            .map_err(|_| PriceLevelFormError::InvalidBasePriceLevelId)?;

        let price_modifier = match value.price_modifier_kind {
            PriceModifierKind::Percent => {
                if (-100..=100).contains(&value.price_modifier) {
                    value.price_modifier
                } else {
                    return Err(PriceLevelFormError::InvalidPriceModifier);
                }
            }
            PriceModifierKind::Fixed => {
                if (-1_000_000..=1_000_000).contains(&value.price_modifier) {
                    value.price_modifier
                } else {
                    return Err(PriceLevelFormError::InvalidPriceModifier);
                }
            }
        };

        let excluded_category_ids: Vec<CategoryId> = value
            .excluded_category_ids
            .into_iter()
            .map(CategoryId::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PriceLevelFormError::InvalidExcludedCategoryId)?;

        let excluded_product_ids: Vec<ProductId> = value
            .excluded_product_ids
            .into_iter()
            .map(ProductId::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PriceLevelFormError::InvalidExcludedProductId)?;

        let included_product_ids: Vec<ProductId> = value
            .included_product_ids
            .into_iter()
            .map(ProductId::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PriceLevelFormError::InvalidIncludedProductId)?;

        let modifier_input = PriceLevelModifierInput {
            base_price_level_id,
            price_modifier,
            price_modifier_kind: value.price_modifier_kind,
            excluded_category_ids,
            excluded_product_ids,
            included_product_ids,
        };

        Ok(Self {
            name,
            default: value.default,
            modifier_input,
        })
    }
}

/// Form payload emitted when submitting the "Edit price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct EditPriceLevelForm {
    /// Updated name entered by the user.
    #[validate(length(min = 1, message = "Название уровня цен обязательно."))]
    pub name: String,
    /// Updated default flag for the price level.
    #[serde(default)]
    pub default: bool,
}

/// Normalized payload for updating a price level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPriceLevelPayload {
    pub name: PriceLevelName,
    pub default: bool,
}

impl TryFrom<EditPriceLevelForm> for EditPriceLevelPayload {
    type Error = PriceLevelFormError;

    fn try_from(value: EditPriceLevelForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let name = PriceLevelName::new(value.name)
            .map_err(|_| PriceLevelFormError::InvalidPriceLevelName)?;

        Ok(Self {
            name,
            default: value.default,
        })
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
                    "name" => Cow::Borrowed("name"),
                    "phone" => Cow::Borrowed("phone"),
                    "public_id" => Cow::Borrowed("public_id"),
                    "price_level_id" => Cow::Borrowed("price_level_id"),
                    "base_price_level_id" => Cow::Borrowed("base_price_level_id"),
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
    fn add_price_level_form_sanitizes_and_converts() {
        let form = AddPriceLevelForm {
            name: "  Premium\tLevel  ".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let payload: AddPriceLevelPayload = form.try_into().expect("expected success");

        assert_eq!(payload.name.as_str(), "Premium\tLevel");
        assert_eq!(payload.modifier_input.base_price_level_id.get(), 1);
        assert_eq!(payload.modifier_input.price_modifier, 10);
        assert_eq!(
            payload.modifier_input.price_modifier_kind,
            PriceModifierKind::Percent
        );
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
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let result: PriceLevelFormResult<AddPriceLevelPayload> = form.try_into();

        assert!(matches!(
            result,
            Err(PriceLevelFormError::InvalidPriceLevelName)
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
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let result: PriceLevelFormResult<AddPriceLevelPayload> = form.try_into();

        assert!(matches!(
            result,
            Err(PriceLevelFormError::InvalidPriceModifier)
        ));
    }

    #[test]
    fn edit_price_level_form_sanitizes_and_converts() {
        let form = EditPriceLevelForm {
            name: "  VIP ".to_string(),
            default: true,
        };

        let payload: EditPriceLevelPayload = form.try_into().expect("expected success");

        assert_eq!(payload.name.as_str(), "VIP");
        assert!(payload.default);
    }

    #[test]
    fn edit_price_level_form_rejects_empty() {
        let form = EditPriceLevelForm {
            name: " ".to_string(),
            default: false,
        };

        let result: PriceLevelFormResult<EditPriceLevelPayload> = form.try_into();

        assert!(matches!(
            result,
            Err(PriceLevelFormError::InvalidPriceLevelName)
        ));
    }
}
