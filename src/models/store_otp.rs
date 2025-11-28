//! Diesel model for storefront OTP records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    store_otp::{NewStoreOtp as DomainNewStoreOtp, StoreOtp as DomainStoreOtp},
    types::{HubId, OtpCode, PhoneNumber, TypeConstraintError},
};

/// Database representation of a storefront OTP record.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(primary_key(hub_id, phone))]
#[diesel(table_name = crate::schema::store_otps)]
pub struct StoreOtp {
    pub hub_id: i32,
    pub phone: String,
    pub code: String,
    pub expires_at: NaiveDateTime,
    pub last_sent_at: NaiveDateTime,
}

/// Payload for inserting a new storefront OTP record.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::store_otps)]
pub struct NewStoreOtp {
    pub hub_id: i32,
    pub phone: String,
    pub code: String,
    pub expires_at: NaiveDateTime,
    pub last_sent_at: NaiveDateTime,
}

impl TryFrom<StoreOtp> for DomainStoreOtp {
    type Error = TypeConstraintError;

    fn try_from(value: StoreOtp) -> Result<Self, Self::Error> {
        Ok(Self {
            hub_id: HubId::new(value.hub_id)?,
            phone: PhoneNumber::new(value.phone)?,
            code: OtpCode::new(value.code)?,
            expires_at: value.expires_at,
            last_sent_at: value.last_sent_at,
        })
    }
}

impl From<&DomainNewStoreOtp> for NewStoreOtp {
    fn from(value: &DomainNewStoreOtp) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            phone: value.phone.as_str().to_string(),
            code: value.code.as_str().to_string(),
            expires_at: value.expires_at,
            last_sent_at: value.last_sent_at,
        }
    }
}
