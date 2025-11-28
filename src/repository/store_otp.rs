use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::store_otp::{NewStoreOtp as DomainNewStoreOtp, StoreOtp as DomainStoreOtp};
use crate::domain::types::TypeConstraintError;
use crate::models::store_otp::{NewStoreOtp as DbNewStoreOtp, StoreOtp as DbStoreOtp};
use crate::repository::{DieselRepository, StoreOtpRepository};

fn map_type_error(
    err: TypeConstraintError,
) -> pushkind_common::repository::errors::RepositoryError {
    pushkind_common::repository::errors::RepositoryError::Unexpected(format!(
        "Invalid store OTP data: {err}"
    ))
}

impl StoreOtpRepository for DieselRepository {
    fn get_store_otp(&self, hub_id: i32, phone: &str) -> RepositoryResult<Option<DomainStoreOtp>> {
        use crate::schema::store_otps;

        let mut conn = self.conn()?;
        let record = store_otps::table
            .filter(store_otps::hub_id.eq(hub_id))
            .filter(store_otps::phone.eq(phone))
            .first::<DbStoreOtp>(&mut conn)
            .optional()?;

        record
            .map(DomainStoreOtp::try_from)
            .transpose()
            .map_err(map_type_error)
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

        DomainStoreOtp::try_from(stored).map_err(map_type_error)
    }

    fn delete_store_otp(&self, hub_id: i32, phone: &str) -> RepositoryResult<()> {
        use crate::schema::store_otps;

        let mut conn = self.conn()?;
        diesel::delete(
            store_otps::table
                .filter(store_otps::hub_id.eq(hub_id))
                .filter(store_otps::phone.eq(phone)),
        )
        .execute(&mut conn)?;

        Ok(())
    }
}
