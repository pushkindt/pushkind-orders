use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::{
    domain::order::OrderStatus,
    domain::types::{
        OrderConsignee, OrderDeliveryNotes, OrderNotes, OrderPayer, OrderReference,
        OrderShippingAddress,
    },
};

/// Result type returned by the order form helpers.
pub type OrderFormResult<T> = Result<T, EditOrderFormError>;

/// Errors that can occur while processing the edit order form.
#[derive(Debug, Error)]
pub enum EditOrderFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The submitted status value is not recognised.
    #[error("Статус заказа указан неверно.")]
    InvalidStatus,
    /// The provided reference value could not be converted.
    #[error("Внешний номер заказа указан неверно.")]
    InvalidReference,
    /// The provided notes value could not be converted.
    #[error("Заметки заказа указаны неверно.")]
    InvalidNotes,
    /// The provided shipping address could not be converted.
    #[error("Адрес доставки указан неверно.")]
    InvalidShippingAddress,
    /// The provided consignee value could not be converted.
    #[error("Получатель указан неверно.")]
    InvalidConsignee,
    /// The provided delivery instructions could not be converted.
    #[error("Инструкции по доставке указаны неверно.")]
    InvalidDeliveryNotes,
    /// The provided payer value could not be converted.
    #[error("Плательщик указан неверно.")]
    InvalidPayer,
}

/// Payload emitted when submitting the edit order modal.
#[derive(Debug, Deserialize, Validate)]
pub struct EditOrderForm {
    /// Identifier of the order being updated.
    #[validate(range(min = 1))]
    pub order_id: i32,
    /// Selected status for the order.
    #[validate(length(min = 1))]
    pub status: String,
    /// Optional external reference displayed to customers.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub reference: Option<String>,
    /// Optional operator notes for the order.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub notes: Option<String>,
    /// Optional shipping address override.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub shipping_address: Option<String>,
    /// Optional consignee information.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub consignee: Option<String>,
    /// Optional delivery instructions.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub delivery_notes: Option<String>,
    /// Optional payer description.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub payer: Option<String>,
}

/// Normalized payload for editing orders.
#[derive(Debug, Clone)]
pub struct EditOrderPayload {
    pub status: OrderStatus,
    pub reference: Option<OrderReference>,
    pub notes: Option<OrderNotes>,
    pub shipping_address: Option<OrderShippingAddress>,
    pub consignee: Option<OrderConsignee>,
    pub delivery_notes: Option<OrderDeliveryNotes>,
    pub payer: Option<OrderPayer>,
}

impl TryFrom<EditOrderForm> for EditOrderPayload {
    type Error = EditOrderFormError;

    fn try_from(value: EditOrderForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let status = OrderStatus::try_from(value.status.trim())
            .map_err(|_| EditOrderFormError::InvalidStatus)?;

        let reference = value
            .reference
            .map(|value| {
                OrderReference::new(value).map_err(|_| EditOrderFormError::InvalidReference)
            })
            .transpose()?;

        let notes = value
            .notes
            .map(|value| OrderNotes::new(value).map_err(|_| EditOrderFormError::InvalidNotes))
            .transpose()?;

        let shipping_address = value
            .shipping_address
            .map(|value| {
                OrderShippingAddress::new(value)
                    .map_err(|_| EditOrderFormError::InvalidShippingAddress)
            })
            .transpose()?;

        let consignee = value
            .consignee
            .map(|value| {
                OrderConsignee::new(value).map_err(|_| EditOrderFormError::InvalidConsignee)
            })
            .transpose()?;

        let delivery_notes = value
            .delivery_notes
            .map(|value| {
                OrderDeliveryNotes::new(value).map_err(|_| EditOrderFormError::InvalidDeliveryNotes)
            })
            .transpose()?;

        let payer = value
            .payer
            .map(|value| OrderPayer::new(value).map_err(|_| EditOrderFormError::InvalidPayer))
            .transpose()?;

        Ok(Self {
            status,
            reference,
            notes,
            shipping_address,
            consignee,
            delivery_notes,
            payer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_order_form_converts_and_sanitizes_values() {
        let form = EditOrderForm {
            order_id: 1,
            status: "Processing".to_string(),
            reference: Some(" REF-1 ".to_string()),
            notes: Some("  notes  ".to_string()),
            shipping_address: Some("  Address  ".to_string()),
            consignee: Some("  Recipient  ".to_string()),
            delivery_notes: Some("  Leave by door  ".to_string()),
            payer: Some("  Company  ".to_string()),
        };

        let updates: EditOrderPayload = form.try_into().expect("expected conversion to succeed");

        assert_eq!(updates.status, OrderStatus::Processing);
        assert_eq!(
            updates.reference.as_ref().map(|value| value.as_str()),
            Some("REF-1")
        );
        assert_eq!(
            updates.notes.as_ref().map(|value| value.as_str()),
            Some("notes")
        );
        assert_eq!(
            updates
                .shipping_address
                .as_ref()
                .map(|value| value.as_str()),
            Some("Address")
        );
        assert_eq!(
            updates.consignee.as_ref().map(|value| value.as_str()),
            Some("Recipient")
        );
        assert_eq!(
            updates.delivery_notes.as_ref().map(|value| value.as_str()),
            Some("Leave by door")
        );
        assert_eq!(
            updates.payer.as_ref().map(|value| value.as_str()),
            Some("Company")
        );
    }

    #[test]
    fn edit_order_form_rejects_invalid_status() {
        let form = EditOrderForm {
            order_id: 1,
            status: "Unknown".to_string(),
            reference: None,
            notes: None,
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        };

        let result: OrderFormResult<EditOrderPayload> = form.try_into();

        assert!(matches!(result, Err(EditOrderFormError::InvalidStatus)));
    }
}
