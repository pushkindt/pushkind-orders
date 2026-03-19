//! Customer repository implementation with Diesel.

use diesel::dsl::{exists, select};
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::customer::UpdateCustomer;
use crate::{
    domain::customer::{Customer as DomainCustomer, NewCustomer as DomainNewCustomer},
    domain::types::{CustomerId, HubId, PhoneNumber, PriceLevelId, PublicId},
    models::customer::{
        Customer as DbCustomer, NewCustomer as DbNewCustomer, UpdateCustomer as DbUpdateCustomer,
    },
    repository::{CustomerListQuery, CustomerReader, CustomerWriter, DieselRepository},
};

impl CustomerReader for DieselRepository {
    fn get_customer_by_id(
        &self,
        id: CustomerId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainCustomer>> {
        use crate::schema::customers;

        let mut conn = self.conn()?;
        let customer = customers::table
            .filter(customers::id.eq(id.get()))
            .filter(customers::hub_id.eq(hub_id.get()))
            .first::<DbCustomer>(&mut conn)
            .optional()?;

        Ok(customer.map(DomainCustomer::try_from).transpose()?)
    }

    fn get_customer_by_phone(
        &self,
        phone: &PhoneNumber,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainCustomer>> {
        use crate::schema::customers;

        let mut conn = self.conn()?;
        let customer = customers::table
            .filter(customers::hub_id.eq(hub_id.get()))
            .filter(customers::phone.eq(phone.as_str()))
            .first::<DbCustomer>(&mut conn)
            .optional()?;

        Ok(customer.map(DomainCustomer::try_from).transpose()?)
    }

    fn get_customer_by_public_id(
        &self,
        public_id: &PublicId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainCustomer>> {
        use crate::schema::customers;

        let mut conn = self.conn()?;
        let customer = customers::table
            .filter(customers::hub_id.eq(hub_id.get()))
            .filter(customers::public_id.eq(public_id.as_str()))
            .first::<DbCustomer>(&mut conn)
            .optional()?;

        Ok(customer.map(DomainCustomer::try_from).transpose()?)
    }

    fn list_customers(
        &self,
        query: CustomerListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainCustomer>)> {
        use crate::schema::customers;

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = customers::table
                .filter(customers::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(term) = query.search.as_ref() {
                let pattern = format!("%{}%", term);
                items = items.filter(
                    customers::name
                        .like(pattern.clone())
                        .or(customers::phone.like(pattern)),
                );
            }

            if let Some(price_level_id) = query.price_level_id {
                items = items.filter(customers::price_level_id.eq(price_level_id.get()));
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder().order(customers::created_at.desc());

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_customers = items.load::<DbCustomer>(&mut conn)?;

        if db_customers.is_empty() {
            return Ok((total, Vec::new()));
        }

        let customers = db_customers
            .into_iter()
            .map(DomainCustomer::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((total, customers))
    }
}

impl CustomerWriter for DieselRepository {
    fn create_customer(
        &self,
        new_customer: &DomainNewCustomer,
    ) -> RepositoryResult<DomainCustomer> {
        use crate::schema::customers;

        let mut conn = self.conn()?;

        if let Some(level_id) = new_customer.price_level_id {
            ensure_price_level_with_hub(&mut conn, new_customer.hub_id.get(), level_id.get())?;
        }

        let db_new = DbNewCustomer::from(new_customer);

        let created = diesel::insert_into(customers::table)
            .values(&db_new)
            .get_result::<DbCustomer>(&mut conn)?;

        Ok(DomainCustomer::try_from(created)?)
    }

    fn assign_price_level_to_customers(
        &self,
        hub_id: HubId,
        customer_ids: &[CustomerId],
        price_level_id: Option<PriceLevelId>,
    ) -> RepositoryResult<()> {
        use crate::schema::customers;

        if customer_ids.is_empty() {
            return Ok(());
        }

        let raw_customer_ids: Vec<i32> = customer_ids.iter().map(|id| id.get()).collect();
        let price_level_id_raw = price_level_id.map(|id| id.get());

        let mut conn = self.conn()?;

        if let Some(level_id) = price_level_id_raw {
            ensure_price_level_with_hub(&mut conn, hub_id.get(), level_id)?;
        }

        let target = customers::table
            .filter(customers::hub_id.eq(hub_id.get()))
            .filter(customers::id.eq_any(&raw_customer_ids));

        let updated = diesel::update(target)
            .set(customers::price_level_id.eq(price_level_id_raw))
            .execute(&mut conn)?;

        if updated != raw_customer_ids.len() {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    fn update_customer(
        &self,
        customer_id: CustomerId,
        hub_id: HubId,
        updates: &UpdateCustomer,
    ) -> RepositoryResult<DomainCustomer> {
        use crate::schema::customers;

        let mut conn = self.conn()?;

        if let Some(level_id) = updates.price_level_id {
            ensure_price_level_with_hub(&mut conn, hub_id.get(), level_id.get())?;
        }

        let target = customers::table
            .filter(customers::id.eq(customer_id.get()))
            .filter(customers::hub_id.eq(hub_id.get()));

        let updates = DbUpdateCustomer::from(updates);

        let updated = diesel::update(target)
            .set(updates)
            .get_result::<DbCustomer>(&mut conn)?;

        Ok(DomainCustomer::try_from(updated)?)
    }
}

/// Verify that a price level exists and belongs to the specified hub.
fn ensure_price_level_with_hub(
    conn: &mut SqliteConnection,
    hub_id: i32,
    price_level_id: i32,
) -> RepositoryResult<()> {
    use crate::schema::price_levels;

    let exists: bool = select(exists(
        price_levels::table
            .filter(price_levels::id.eq(price_level_id))
            .filter(price_levels::hub_id.eq(hub_id)),
    ))
    .get_result(conn)?;

    if exists {
        Ok(())
    } else {
        Err(RepositoryError::NotFound)
    }
}
