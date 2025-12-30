//! Storefront OTP repository implementation with Diesel.

use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::store_otp::{NewStoreOtp as DomainNewStoreOtp, StoreOtp as DomainStoreOtp};
use crate::domain::types::{HubId, PhoneNumber};
use crate::models::store_otp::{NewStoreOtp as DbNewStoreOtp, StoreOtp as DbStoreOtp};
use crate::repository::{DieselRepository, StoreOtpRepository};

impl StoreOtpRepository for DieselRepository {
    fn get_store_otp(
        &self,
        hub_id: HubId,
        phone: &PhoneNumber,
    ) -> RepositoryResult<Option<DomainStoreOtp>> {
        use crate::schema::store_otps;

        let mut conn = self.conn()?;
        let record = store_otps::table
            .filter(store_otps::hub_id.eq(hub_id.get()))
            .filter(store_otps::phone.eq(phone.as_str()))
            .first::<DbStoreOtp>(&mut conn)
            .optional()?;

        Ok(record.map(DomainStoreOtp::try_from).transpose()?)
    }

    fn upsert_store_otp(&self, new_otp: &DomainNewStoreOtp) -> RepositoryResult<DomainStoreOtp> {
        use crate::schema::store_otps;

        let mut conn = self.conn()?;
        let db_new = DbNewStoreOtp::from(new_otp);

        let stored = diesel::insert_into(store_otps::table)
            .values(&db_new)
            .on_conflict((store_otps::hub_id, store_otps::phone))
            .do_update()
            .set((
                store_otps::code.eq(db_new.code.clone()),
                store_otps::expires_at.eq(db_new.expires_at),
                store_otps::last_sent_at.eq(db_new.last_sent_at),
            ))
            .returning(DbStoreOtp::as_returning())
            .get_result::<DbStoreOtp>(&mut conn)?;

        Ok(DomainStoreOtp::try_from(stored)?)
    }

    fn delete_store_otp(&self, hub_id: HubId, phone: &PhoneNumber) -> RepositoryResult<()> {
        use crate::schema::store_otps;

        let mut conn = self.conn()?;
        diesel::delete(
            store_otps::table
                .filter(store_otps::hub_id.eq(hub_id.get()))
                .filter(store_otps::phone.eq(phone.as_str())),
        )
        .execute(&mut conn)?;

        Ok(())
    }
}
