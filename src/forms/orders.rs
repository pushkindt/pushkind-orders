use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::Deserialize;
use validator::Validate;

use crate::{
    domain::order::OrderStatus,
    domain::types::{
        OrderConsignee, OrderDeliveryNotes, OrderNotes, OrderPayer, OrderReference,
        OrderShippingAddress, ProductId, ProductQuantity,
    },
    forms::FormError,
};

/// Result type returned by the order form helpers.
pub type OrderFormResult<T> = Result<T, FormError>;

/// Result type returned by the order approvals form helpers.
pub type OrderApprovalsFormResult<T> = Result<T, FormError>;

/// Payload emitted when submitting the edit order modal.
#[derive(Debug, Deserialize, Validate)]
pub struct EditOrderForm {
    /// Identifier of the order being updated.
    #[validate(range(min = 1, message = "Идентификатор заказа указан неверно."))]
    pub order_id: i32,
    /// Selected status for the order.
    #[validate(length(min = 1, message = "Выберите статус заказа."))]
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

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOrderApprovalItemForm {
    #[validate(range(min = 1, message = "Выберите товар."))]
    pub product_id: i32,
    #[validate(range(min = 1, message = "Количество должно быть положительным целым."))]
    pub approved_quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderApprovalsForm {
    pub approvals: Vec<UpdateOrderApprovalItemForm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOrderApprovalItemPayload {
    pub product_id: ProductId,
    pub approved_quantity: ProductQuantity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOrderApprovalsPayload {
    pub approvals: Vec<UpdateOrderApprovalItemPayload>,
}

impl TryFrom<EditOrderForm> for EditOrderPayload {
    type Error = FormError;

    fn try_from(value: EditOrderForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let status = OrderStatus::try_from(value.status.trim())
            .map_err(|_| FormError::InvalidOrderStatus)?;

        let reference = value
            .reference
            .map(|value| OrderReference::new(value).map_err(|_| FormError::InvalidOrderReference))
            .transpose()?;

        let notes = value
            .notes
            .map(|value| OrderNotes::new(value).map_err(|_| FormError::InvalidOrderNotes))
            .transpose()?;

        let shipping_address = value
            .shipping_address
            .map(|value| {
                OrderShippingAddress::new(value).map_err(|_| FormError::InvalidOrderShippingAddress)
            })
            .transpose()?;

        let consignee = value
            .consignee
            .map(|value| OrderConsignee::new(value).map_err(|_| FormError::InvalidOrderConsignee))
            .transpose()?;

        let delivery_notes = value
            .delivery_notes
            .map(|value| {
                OrderDeliveryNotes::new(value).map_err(|_| FormError::InvalidOrderDeliveryNotes)
            })
            .transpose()?;

        let payer = value
            .payer
            .map(|value| OrderPayer::new(value).map_err(|_| FormError::InvalidOrderPayer))
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

impl TryFrom<UpdateOrderApprovalsForm> for UpdateOrderApprovalsPayload {
    type Error = FormError;

    fn try_from(value: UpdateOrderApprovalsForm) -> Result<Self, Self::Error> {
        if value.approvals.is_empty() {
            return Err(FormError::EmptyOrderApprovals);
        }

        let approvals = value
            .approvals
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                item.validate()
                    .map_err(|errors| FormError::PrefixedValidation {
                        prefix: indexed_field_prefix(index),
                        errors,
                    })?;

                let product_id = ProductId::new(item.product_id)
                    .map_err(|_| FormError::InvalidOrderApprovalProductId { index })?;
                let approved_quantity = ProductQuantity::new(item.approved_quantity)
                    .map_err(|_| FormError::InvalidOrderApprovalQuantity { index })?;

                Ok(UpdateOrderApprovalItemPayload {
                    product_id,
                    approved_quantity,
                })
            })
            .collect::<Result<Vec<_>, FormError>>()?;

        Ok(Self { approvals })
    }
}

fn indexed_field_prefix(index: usize) -> String {
    format!("approvals.{index}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_errors(error: &FormError) -> Vec<(String, String)> {
        let mut field_errors = error
            .field_errors()
            .into_iter()
            .map(|error| (error.field.to_string(), error.message.into_owned()))
            .collect::<Vec<_>>();
        field_errors.sort();
        field_errors
    }

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

        assert!(matches!(result, Err(FormError::InvalidOrderStatus)));
    }

    #[test]
    fn edit_order_form_uses_localized_validation_messages() {
        let form = EditOrderForm {
            order_id: 0,
            status: String::new(),
            reference: None,
            notes: None,
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        };

        let error = FormError::from(form.validate().expect_err("form should be invalid"));

        assert_eq!(
            field_errors(&error),
            vec![
                (
                    "order_id".to_string(),
                    "Идентификатор заказа указан неверно.".to_string(),
                ),
                ("status".to_string(), "Выберите статус заказа.".to_string()),
            ]
        );
    }

    #[test]
    fn approvals_form_converts_to_strongly_typed_payload() {
        let form = UpdateOrderApprovalsForm {
            approvals: vec![UpdateOrderApprovalItemForm {
                product_id: 5,
                approved_quantity: 3,
            }],
        };

        let payload: UpdateOrderApprovalsPayload = form
            .try_into()
            .expect("approvals payload conversion should succeed");

        assert_eq!(payload.approvals.len(), 1);
        assert_eq!(payload.approvals[0].product_id.get(), 5);
        assert_eq!(payload.approvals[0].approved_quantity.get(), 3);
    }

    #[test]
    fn approvals_form_rejects_empty_payload() {
        let form = UpdateOrderApprovalsForm {
            approvals: Vec::new(),
        };

        let result: OrderApprovalsFormResult<UpdateOrderApprovalsPayload> = form.try_into();

        let error = result.expect_err("empty approvals should fail");
        assert_eq!(
            field_errors(&error),
            vec![(
                "approvals".to_string(),
                "Не выбраны позиции для обновления.".to_string(),
            )]
        );
    }

    #[test]
    fn approvals_form_reports_indexed_field_errors() {
        let form = UpdateOrderApprovalsForm {
            approvals: vec![UpdateOrderApprovalItemForm {
                product_id: 1,
                approved_quantity: 0,
            }],
        };

        let result: OrderApprovalsFormResult<UpdateOrderApprovalsPayload> = form.try_into();

        let error = result.expect_err("invalid approvals should fail");
        assert_eq!(
            field_errors(&error),
            vec![(
                "approvals.0.approved_quantity".to_string(),
                "Количество должно быть положительным целым.".to_string(),
            )]
        );
    }
}
