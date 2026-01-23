//! Vendor repository implementation with Diesel.

use diesel::OptionalExtension;
use diesel::dsl::{exists, select};
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::types::{HubId, OrderId, UserId, VendorId};
use crate::domain::vendor::{
    NewVendor as DomainNewVendor, UpdateVendor as DomainUpdateVendor, Vendor as DomainVendor,
    VendorListQuery,
};
use crate::models::vendor::{
    NewVendor as DbNewVendor, UpdateVendor as DbUpdateVendor, Vendor as DbVendor,
};
use crate::models::vendor_order::NewVendorOrder as DbNewVendorOrder;
use crate::models::vendor_user::NewVendorUser as DbNewVendorUser;
use crate::repository::{
    DieselRepository, VendorOrderReader, VendorOrderWriter, VendorReader, VendorUserReader,
    VendorUserWriter, VendorWriter,
};

impl VendorReader for DieselRepository {
    fn get_vendor_by_id(
        &self,
        vendor_id: VendorId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainVendor>> {
        use crate::schema::vendors;

        let mut conn = self.conn()?;

        let vendor = vendors::table
            .filter(vendors::id.eq(vendor_id.get()))
            .filter(vendors::hub_id.eq(hub_id.get()))
            .first::<DbVendor>(&mut conn)
            .optional()?;

        let vendor = vendor.map(DomainVendor::try_from).transpose()?;

        Ok(vendor)
    }

    fn list_vendors(&self, query: VendorListQuery) -> RepositoryResult<(usize, Vec<DomainVendor>)> {
        use crate::schema::vendors;

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = vendors::table
                .filter(vendors::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(search) = query.search.as_ref() {
                let pattern = format!("%{}%", search);
                items = items.filter(vendors::name.like(pattern));
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder().order(vendors::name.asc());

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let vendors = items.load::<DbVendor>(&mut conn)?;
        let vendors = vendors
            .into_iter()
            .map(DomainVendor::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((total, vendors))
    }
}

impl VendorWriter for DieselRepository {
    fn create_vendor(&self, new_vendor: &DomainNewVendor) -> RepositoryResult<DomainVendor> {
        use crate::schema::vendors;

        let mut conn = self.conn()?;
        let insertable = DbNewVendor::from(new_vendor);

        let created = diesel::insert_into(vendors::table)
            .values(&insertable)
            .get_result::<DbVendor>(&mut conn)?;

        Ok(DomainVendor::try_from(created)?)
    }

    fn update_vendor(
        &self,
        vendor_id: VendorId,
        hub_id: HubId,
        updates: &DomainUpdateVendor,
    ) -> RepositoryResult<DomainVendor> {
        use crate::schema::vendors;

        let mut conn = self.conn()?;
        let db_updates = DbUpdateVendor::from(updates);

        let target = vendors::table
            .filter(vendors::id.eq(vendor_id.get()))
            .filter(vendors::hub_id.eq(hub_id.get()));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbVendor>(&mut conn)?;

        Ok(DomainVendor::try_from(updated)?)
    }

    fn delete_vendor(&self, vendor_id: VendorId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::vendors;

        let mut conn = self.conn()?;
        let target = vendors::table
            .filter(vendors::id.eq(vendor_id.get()))
            .filter(vendors::hub_id.eq(hub_id.get()));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}

impl VendorUserReader for DieselRepository {
    fn get_vendor_for_user(
        &self,
        user_id: UserId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<VendorId>> {
        use crate::schema::{users, vendor_user, vendors};

        let mut conn = self.conn()?;

        let vendor_id = vendor_user::table
            .inner_join(vendors::table.on(vendors::id.eq(vendor_user::vendor_id)))
            .inner_join(users::table.on(users::id.eq(vendor_user::user_id)))
            .filter(users::id.eq(user_id.get()))
            .filter(users::hub_id.eq(hub_id.get()))
            .filter(vendors::hub_id.eq(hub_id.get()))
            .select(vendor_user::vendor_id)
            .first::<i32>(&mut conn)
            .optional()?;

        vendor_id.map(VendorId::new).transpose().map_err(Into::into)
    }
}

impl VendorUserWriter for DieselRepository {
    fn assign_user_to_vendor(
        &self,
        user_id: UserId,
        vendor_id: VendorId,
        hub_id: HubId,
    ) -> RepositoryResult<()> {
        use crate::schema::{users, vendor_user, vendors};

        let mut conn = self.conn()?;

        conn.transaction::<(), RepositoryError, _>(|conn| {
            let vendor_exists: bool = select(exists(
                vendors::table
                    .filter(vendors::id.eq(vendor_id.get()))
                    .filter(vendors::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;
            if !vendor_exists {
                return Err(RepositoryError::NotFound);
            }

            let user_exists: bool = select(exists(
                users::table
                    .filter(users::id.eq(user_id.get()))
                    .filter(users::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;
            if !user_exists {
                return Err(RepositoryError::NotFound);
            }

            diesel::delete(vendor_user::table.filter(vendor_user::user_id.eq(user_id.get())))
                .execute(conn)?;

            let insertable = DbNewVendorUser {
                vendor_id: vendor_id.get(),
                user_id: user_id.get(),
            };

            diesel::insert_into(vendor_user::table)
                .values(&insertable)
                .execute(conn)?;

            Ok(())
        })
    }

    fn clear_vendor_for_user(&self, user_id: UserId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{users, vendor_user};

        let mut conn = self.conn()?;

        let user_exists: bool = select(exists(
            users::table
                .filter(users::id.eq(user_id.get()))
                .filter(users::hub_id.eq(hub_id.get())),
        ))
        .get_result(&mut conn)?;
        if !user_exists {
            return Err(RepositoryError::NotFound);
        }

        diesel::delete(vendor_user::table.filter(vendor_user::user_id.eq(user_id.get())))
            .execute(&mut conn)?;

        Ok(())
    }
}

impl VendorOrderReader for DieselRepository {
    fn get_vendor_for_order(
        &self,
        order_id: OrderId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<VendorId>> {
        use crate::schema::{orders, vendor_order, vendors};

        let mut conn = self.conn()?;

        let vendor_id = vendor_order::table
            .inner_join(orders::table.on(orders::id.eq(vendor_order::order_id)))
            .inner_join(vendors::table.on(vendors::id.eq(vendor_order::vendor_id)))
            .filter(orders::id.eq(order_id.get()))
            .filter(orders::hub_id.eq(hub_id.get()))
            .filter(vendors::hub_id.eq(hub_id.get()))
            .select(vendor_order::vendor_id)
            .first::<i32>(&mut conn)
            .optional()?;

        vendor_id.map(VendorId::new).transpose().map_err(Into::into)
    }
}

impl VendorOrderWriter for DieselRepository {
    fn associate_order_with_vendor(
        &self,
        order_id: OrderId,
        vendor_id: VendorId,
        hub_id: HubId,
    ) -> RepositoryResult<()> {
        use crate::schema::{orders, vendor_order, vendors};

        let mut conn = self.conn()?;

        conn.transaction::<(), RepositoryError, _>(|conn| {
            let vendor_exists: bool = select(exists(
                vendors::table
                    .filter(vendors::id.eq(vendor_id.get()))
                    .filter(vendors::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;
            if !vendor_exists {
                return Err(RepositoryError::NotFound);
            }

            let order_exists: bool = select(exists(
                orders::table
                    .filter(orders::id.eq(order_id.get()))
                    .filter(orders::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;
            if !order_exists {
                return Err(RepositoryError::NotFound);
            }

            diesel::delete(vendor_order::table.filter(vendor_order::order_id.eq(order_id.get())))
                .execute(conn)?;

            let insertable = DbNewVendorOrder {
                vendor_id: vendor_id.get(),
                order_id: order_id.get(),
            };

            diesel::insert_into(vendor_order::table)
                .values(&insertable)
                .execute(conn)?;

            Ok(())
        })
    }

    fn clear_vendor_for_order(&self, order_id: OrderId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{orders, vendor_order};

        let mut conn = self.conn()?;

        let order_exists: bool = select(exists(
            orders::table
                .filter(orders::id.eq(order_id.get()))
                .filter(orders::hub_id.eq(hub_id.get())),
        ))
        .get_result(&mut conn)?;
        if !order_exists {
            return Err(RepositoryError::NotFound);
        }

        diesel::delete(vendor_order::table.filter(vendor_order::order_id.eq(order_id.get())))
            .execute(&mut conn)?;

        Ok(())
    }
}
