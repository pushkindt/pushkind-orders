use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::domain::types::{
    OrderConsignee, OrderDeliveryNotes, OrderPayer, OrderShippingAddress, TypeConstraintError,
};
use crate::forms::sanitize_text;

/// Result type used by store form helpers.
pub type StoreFormResult<T> = Result<T, StoreFormError>;

/// Errors emitted when processing store-facing form payloads.
#[derive(Debug, Error)]
pub enum StoreFormError {
    /// Validation failures bubbled up from `validator`.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
}

/// Payload describing a single line in a storefront order request.
#[derive(Debug, Clone, Deserialize, Validate, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOrderLinePayload {
    /// Identifier of the product being ordered.
    #[validate(range(min = 1))]
    pub product_id: i32,
    /// Quantity requested for the product.
    #[validate(range(min = 1))]
    pub quantity: i32,
}

impl StoreOrderLinePayload {
    /// Validate and normalize the payload, ensuring a positive quantity.
    pub fn into_request(self) -> StoreFormResult<StoreOrderLineInput> {
        self.validate()?;

        Ok(StoreOrderLineInput {
            product_id: self.product_id,
            quantity: self.quantity,
        })
    }
}

/// Normalized order line forwarded to the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOrderLineInput {
    /// Identifier of the product being ordered.
    pub product_id: i32,
    /// Quantity requested for the product.
    pub quantity: i32,
}

/// Validate a batch of storefront order line payloads.
pub fn validate_store_order_lines(
    payloads: Vec<StoreOrderLinePayload>,
) -> StoreFormResult<Vec<StoreOrderLineInput>> {
    if payloads.is_empty() {
        let mut errors = ValidationErrors::new();
        errors.add("items", ValidationError::new("length"));
        return Err(StoreFormError::Validation(errors));
    }

    payloads
        .into_iter()
        .map(StoreOrderLinePayload::into_request)
        .collect()
}

/// Fields that storefront customers can update on an existing order.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreOrderUpdatePayload {
    #[serde(default)]
    pub shipping_address: Option<Option<String>>,
    #[serde(default)]
    pub consignee: Option<Option<String>>,
    #[serde(default)]
    pub delivery_notes: Option<Option<String>>,
    #[serde(default)]
    pub payer: Option<Option<String>>,
}

/// Sanitized domain representations of updateable storefront order fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoreOrderUpdateValues {
    pub shipping_address: Option<Option<OrderShippingAddress>>,
    pub consignee: Option<Option<OrderConsignee>>,
    pub delivery_notes: Option<Option<OrderDeliveryNotes>>,
    pub payer: Option<Option<OrderPayer>>,
}

/// Errors produced when validating a storefront order update payload.
#[derive(Debug, Error)]
pub enum StoreOrderUpdateError {
    #[error("shippingAddress is invalid")]
    InvalidShippingAddress,
    #[error("consignee is invalid")]
    InvalidConsignee,
    #[error("deliveryNotes is invalid")]
    InvalidDeliveryNotes,
    #[error("payer is invalid")]
    InvalidPayer,
}

impl StoreOrderUpdatePayload {
    /// Normalize and validate the incoming payload for use in the service layer.
    pub fn into_values(self) -> Result<StoreOrderUpdateValues, StoreOrderUpdateError> {
        Ok(StoreOrderUpdateValues {
            shipping_address: parse_optional_field(
                self.shipping_address,
                OrderShippingAddress::new,
                StoreOrderUpdateError::InvalidShippingAddress,
            )?,
            consignee: parse_optional_field(
                self.consignee,
                OrderConsignee::new,
                StoreOrderUpdateError::InvalidConsignee,
            )?,
            delivery_notes: parse_optional_field(
                self.delivery_notes,
                OrderDeliveryNotes::new,
                StoreOrderUpdateError::InvalidDeliveryNotes,
            )?,
            payer: parse_optional_field(
                self.payer,
                OrderPayer::new,
                StoreOrderUpdateError::InvalidPayer,
            )?,
        })
    }
}

fn parse_optional_field<T, F>(
    raw_value: Option<Option<String>>,
    constructor: F,
    error: StoreOrderUpdateError,
) -> Result<Option<Option<T>>, StoreOrderUpdateError>
where
    F: Fn(String) -> Result<T, TypeConstraintError>,
{
    match raw_value {
        None => Ok(None),
        Some(None) => Ok(Some(None)),
        Some(Some(value)) => match sanitize_text(&value) {
            None => Ok(Some(None)),
            Some(trimmed) => match constructor(trimmed) {
                Ok(converted) => Ok(Some(Some(converted))),
                Err(_) => Err(error),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_order_line_payload_accepts_positive_quantity() {
        let payload = StoreOrderLinePayload {
            product_id: 1,
            quantity: 2,
        };

        let normalized = payload.into_request().expect("valid order line");

        assert_eq!(normalized.product_id, 1);
        assert_eq!(normalized.quantity, 2);
    }

    #[test]
    fn store_order_line_payload_rejects_zero_quantity() {
        let payload = StoreOrderLinePayload {
            product_id: 1,
            quantity: 0,
        };

        let result = payload.into_request();

        assert!(matches!(result, Err(StoreFormError::Validation(_))));
    }

    #[test]
    fn validate_store_order_lines_rejects_empty_payloads() {
        let result = validate_store_order_lines(Vec::new());

        assert!(matches!(result, Err(StoreFormError::Validation(_))));
    }

    #[test]
    fn store_order_update_payload_sanitizes_fields() {
        let payload = StoreOrderUpdatePayload {
            shipping_address: Some(Some("  Address  ".to_string())),
            consignee: Some(Some("  Recipient  ".to_string())),
            delivery_notes: Some(Some("  Leave at door  ".to_string())),
            payer: Some(Some("  Company  ".to_string())),
        };

        let values = payload.into_values().expect("valid payload");

        assert_eq!(
            values
                .shipping_address
                .as_ref()
                .and_then(|value| value.as_ref().map(|text| text.as_str())),
            Some("Address")
        );
        assert_eq!(
            values
                .consignee
                .as_ref()
                .and_then(|value| value.as_ref().map(|text| text.as_str())),
            Some("Recipient")
        );
        assert_eq!(
            values
                .delivery_notes
                .as_ref()
                .and_then(|value| value.as_ref().map(|text| text.as_str())),
            Some("Leave at door")
        );
        assert_eq!(
            values
                .payer
                .as_ref()
                .and_then(|value| value.as_ref().map(|text| text.as_str())),
            Some("Company")
        );
    }

    #[test]
    fn store_order_update_payload_handles_clear_values() {
        let payload = StoreOrderUpdatePayload {
            shipping_address: Some(Some("    ".to_string())),
            consignee: Some(None),
            delivery_notes: None,
            payer: None,
        };

        let values = payload.into_values().expect("valid payload");

        assert_eq!(values.shipping_address, Some(None));
        assert_eq!(values.consignee, Some(None));
        assert_eq!(values.delivery_notes, None);
        assert_eq!(values.payer, None);
    }
}
