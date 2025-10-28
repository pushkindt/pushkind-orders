use diesel::dsl::{exists, select};
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::customer::{Customer as DomainCustomer, NewCustomer as DomainNewCustomer},
    models::customer::{Customer as DbCustomer, NewCustomer as DbNewCustomer},
    repository::{CustomerListQuery, CustomerReader, CustomerWriter, DieselRepository},
};

impl CustomerReader for DieselRepository {
    fn get_customer_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainCustomer>> {
        use crate::schema::customers;

        let mut conn = self.conn()?;
        let customer = customers::table
            .filter(customers::id.eq(id))
            .filter(customers::hub_id.eq(hub_id))
            .first::<DbCustomer>(&mut conn)
            .optional()?;

        Ok(customer.map(Into::into))
    }

    fn get_customer_by_email(
        &self,
        email: &str,
        hub_id: i32,
    ) -> RepositoryResult<Option<DomainCustomer>> {
        use crate::schema::customers;

        let normalized_email = email.to_lowercase();

        let mut conn = self.conn()?;
        let customer = customers::table
            .filter(customers::email.eq(normalized_email))
            .filter(customers::hub_id.eq(hub_id))
            .first::<DbCustomer>(&mut conn)
            .optional()?;

        Ok(customer.map(Into::into))
    }

    fn list_customers(
        &self,
        query: CustomerListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainCustomer>)> {
        use crate::schema::customers;

        let mut conn = self.conn()?;

        let mut count_query = customers::table
            .filter(customers::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            count_query = count_query.filter(
                customers::name
                    .like(pattern.clone())
                    .or(customers::email.like(pattern)),
            );
        }

        if let Some(price_level_id) = query.price_level_id {
            count_query = count_query.filter(customers::price_level_id.eq(price_level_id));
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items = customers::table
            .filter(customers::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            items = items.filter(
                customers::name
                    .like(pattern.clone())
                    .or(customers::email.like(pattern)),
            );
        }

        if let Some(price_level_id) = query.price_level_id {
            items = items.filter(customers::price_level_id.eq(price_level_id));
        }

        items = items.order(customers::created_at.desc());

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_customers = items.load::<DbCustomer>(&mut conn)?;

        if db_customers.is_empty() {
            return Ok((total, Vec::new()));
        }

        Ok((total, db_customers.into_iter().map(Into::into).collect()))
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
            ensure_price_level_with_hub(&mut conn, new_customer.hub_id, level_id)?;
        }

        let db_new = DbNewCustomer::from(new_customer);

        let created = diesel::insert_into(customers::table)
            .values(&db_new)
            .get_result::<DbCustomer>(&mut conn)?;

        Ok(created.into())
    }

    fn assign_price_level_to_customers(
        &self,
        hub_id: i32,
        customer_ids: &[i32],
        price_level_id: Option<i32>,
    ) -> RepositoryResult<()> {
        use crate::schema::customers;

        if customer_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn()?;

        if let Some(level_id) = price_level_id {
            ensure_price_level_with_hub(&mut conn, hub_id, level_id)?;
        }

        let target = customers::table
            .filter(customers::hub_id.eq(hub_id))
            .filter(customers::id.eq_any(customer_ids));

        let updated = diesel::update(target)
            .set(customers::price_level_id.eq(price_level_id))
            .execute(&mut conn)?;

        if updated != customer_ids.len() {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}

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
