use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::store_otp::{NewStoreOtp as DomainNewStoreOtp, StoreOtp as DomainStoreOtp};

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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::store_otps)]
pub struct NewStoreOtp {
    pub hub_id: i32,
    pub phone: String,
    pub code: String,
    pub expires_at: NaiveDateTime,
    pub last_sent_at: NaiveDateTime,
}

impl From<StoreOtp> for DomainStoreOtp {
    fn from(value: StoreOtp) -> Self {
        Self {
            hub_id: value.hub_id,
            phone: value.phone,
            code: value.code,
            expires_at: value.expires_at,
            last_sent_at: value.last_sent_at,
        }
    }
}

impl From<&DomainNewStoreOtp> for NewStoreOtp {
    fn from(value: &DomainNewStoreOtp) -> Self {
        Self {
            hub_id: value.hub_id,
            phone: value.phone.clone(),
            code: value.code.clone(),
            expires_at: value.expires_at,
            last_sent_at: value.last_sent_at,
        }
    }
}
