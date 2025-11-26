use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::forms::{PhoneNormalizationError, normalize_phone_to_e164};

/// Maximum length allowed for a phone number provided by storefront clients.
const PHONE_MAX_LEN: usize = 64;
const PHONE_MAX_LEN_VALIDATOR: u64 = PHONE_MAX_LEN as u64;

/// Result type used by store form helpers.
pub type StoreFormResult<T> = Result<T, StoreFormError>;

/// Errors emitted when processing store-facing form payloads.
#[derive(Debug, Error)]
pub enum StoreFormError {
    /// Validation failures bubbled up from `validator`.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// Phone number becomes empty after sanitization.
    #[error("phone number is required")]
    EmptyPhone,
    /// Phone number failed to normalize into E.164.
    #[error("phone number is invalid")]
    InvalidPhone,
    /// OTP contains invalid characters or length.
    #[error("otp must be a 6-digit code")]
    InvalidOtp,
}

/// Payload accepted by the `/auth/otp` endpoint.
#[derive(Debug, Deserialize, Validate)]
pub struct StoreOtpRequestPayload {
    /// Phone number entered by the user.
    #[validate(length(min = 1, max = PHONE_MAX_LEN_VALIDATOR))]
    pub phone: String,
}

impl StoreOtpRequestPayload {
    /// Validate and normalize the payload, trimming the phone field.
    pub fn into_request(self) -> StoreFormResult<StoreOtpRequestInput> {
        self.validate()?;

        let phone = normalize_phone_to_e164(&self.phone).map_err(|err| match err {
            PhoneNormalizationError::Empty => StoreFormError::EmptyPhone,
            PhoneNormalizationError::Invalid => StoreFormError::InvalidPhone,
        })?;

        Ok(StoreOtpRequestInput { phone })
    }
}

/// Normalized OTP request forwarded to the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOtpRequestInput {
    /// Sanitized phone number.
    pub phone: String,
}

/// Payload accepted by the `/auth/otp/verify` endpoint.
#[derive(Debug, Deserialize)]
pub struct StoreOtpVerifyPayload {
    /// Phone number entered by the user.
    pub phone: String,
    /// One-time password entered by the user.
    pub otp: String,
}

impl StoreOtpVerifyPayload {
    /// Validate and normalize the payload, ensuring the OTP contains six digits.
    pub fn into_request(self) -> StoreFormResult<StoreOtpVerifyInput> {
        let phone = normalize_phone_to_e164(&self.phone).map_err(|err| match err {
            PhoneNormalizationError::Empty => StoreFormError::EmptyPhone,
            PhoneNormalizationError::Invalid => StoreFormError::InvalidPhone,
        })?;
        let otp = self.otp.trim();

        if otp.len() != 6 || !otp.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(StoreFormError::InvalidOtp);
        }

        Ok(StoreOtpVerifyInput {
            phone,
            otp: otp.to_string(),
        })
    }
}

/// Normalized OTP verification request forwarded to the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOtpVerifyInput {
    /// Sanitized phone number.
    pub phone: String,
    /// Sanitized one-time password.
    pub otp: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_otp_request_payload_trims_phone() {
        let payload = StoreOtpRequestPayload {
            phone: "  +1 (555) 123-4567  ".to_string(),
        };

        let normalized = payload.into_request().expect("valid payload");

        assert_eq!(normalized.phone, "+15551234567");
    }

    #[test]
    fn store_otp_request_payload_rejects_empty() {
        let payload = StoreOtpRequestPayload {
            phone: "   ".to_string(),
        };

        let result = payload.into_request();

        assert!(matches!(result, Err(StoreFormError::EmptyPhone)));
    }

    #[test]
    fn store_otp_verify_payload_trims_and_accepts_digits() {
        let payload = StoreOtpVerifyPayload {
            phone: " +1222333 ".to_string(),
            otp: " 123456 ".to_string(),
        };

        let normalized = payload.into_request().expect("valid payload");

        assert_eq!(normalized.phone, "+1222333");
        assert_eq!(normalized.otp, "123456");
    }

    #[test]
    fn store_otp_verify_payload_rejects_non_digits() {
        let payload = StoreOtpVerifyPayload {
            phone: "+1222333".to_string(),
            otp: "12a456".to_string(),
        };

        let result = payload.into_request();

        assert!(matches!(result, Err(StoreFormError::InvalidOtp)));
    }

    #[test]
    fn store_otp_request_rejects_invalid_phone() {
        let payload = StoreOtpRequestPayload {
            phone: "abc".to_string(),
        };

        let result = payload.into_request();

        assert!(matches!(result, Err(StoreFormError::InvalidPhone)));
    }

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
}
