//! Storefront OTP domain models for customer authentication.

use chrono::NaiveDateTime;

use crate::domain::types::{HubId, OtpCode, PhoneNumber, TypeConstraintError};

/// Domain representation of an OTP request associated with a storefront customer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOtp {
    /// Hub identifier that owns the OTP record.
    pub hub_id: HubId,
    /// Phone number the OTP was generated for.
    pub phone: PhoneNumber,
    /// Six-digit OTP code.
    pub code: OtpCode,
    /// Timestamp indicating when the OTP expires.
    pub expires_at: NaiveDateTime,
    /// Timestamp capturing when the OTP was last sent.
    pub last_sent_at: NaiveDateTime,
}

/// Payload used to persist a new or updated OTP record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStoreOtp {
    /// Hub identifier that owns the OTP record.
    pub hub_id: HubId,
    /// Phone number the OTP targets.
    pub phone: PhoneNumber,
    /// Six-digit OTP code.
    pub code: OtpCode,
    /// Timestamp indicating when the OTP expires.
    pub expires_at: NaiveDateTime,
    /// Timestamp capturing when the OTP was last sent.
    pub last_sent_at: NaiveDateTime,
}

impl NewStoreOtp {
    /// Construct a new OTP payload from validated value objects.
    #[must_use]
    pub fn new(
        hub_id: HubId,
        phone: PhoneNumber,
        code: OtpCode,
        expires_at: NaiveDateTime,
        last_sent_at: NaiveDateTime,
    ) -> Self {
        Self {
            hub_id,
            phone,
            code,
            expires_at,
            last_sent_at,
        }
    }

    /// Attempt to construct a new OTP payload by enforcing domain constraints.
    pub fn try_new(
        hub_id: i32,
        phone: impl Into<String>,
        code: impl Into<String>,
        expires_at: NaiveDateTime,
        last_sent_at: NaiveDateTime,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            PhoneNumber::new(phone)?,
            OtpCode::new(code)?,
            expires_at,
            last_sent_at,
        ))
    }
}
