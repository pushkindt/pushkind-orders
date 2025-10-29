use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::price_level::{
        NewPriceLevel as DomainNewPriceLevel, PriceLevel as DomainPriceLevel, PriceLevelListQuery,
        UpdatePriceLevel as DomainUpdatePriceLevel,
    },
    models::price_level::{
        NewPriceLevel as DbNewPriceLevel, PriceLevel as DbPriceLevel,
        UpdatePriceLevel as DbUpdatePriceLevel,
    },
    repository::{DieselRepository, PriceLevelReader, PriceLevelWriter},
};

impl PriceLevelReader for DieselRepository {
    fn get_price_level_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<DomainPriceLevel>> {
        use crate::schema::price_levels;

        let mut conn = self.conn()?;
        let price_level = price_levels::table
            .filter(price_levels::id.eq(id))
            .filter(price_levels::hub_id.eq(hub_id))
            .first::<DbPriceLevel>(&mut conn)
            .optional()?;

        Ok(price_level.map(Into::into))
    }

    fn list_price_levels(
        &self,
        query: PriceLevelListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainPriceLevel>)> {
        use crate::schema::price_levels;

        let mut conn = self.conn()?;

        let mut count_query = price_levels::table
            .filter(price_levels::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            count_query = count_query.filter(price_levels::name.like(pattern.clone()));
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items = price_levels::table
            .filter(price_levels::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            items = items.filter(price_levels::name.like(pattern.clone()));
        }

        items = items.order((price_levels::name.asc(), price_levels::created_at.asc()));

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_price_levels = items.load::<DbPriceLevel>(&mut conn)?;

        if db_price_levels.is_empty() {
            return Ok((total, Vec::new()));
        }

        Ok((total, db_price_levels.into_iter().map(Into::into).collect()))
    }
}

impl PriceLevelWriter for DieselRepository {
    fn create_price_level(
        &self,
        new_price_level: &DomainNewPriceLevel,
    ) -> RepositoryResult<DomainPriceLevel> {
        use crate::schema::price_levels;

        let mut conn = self.conn()?;
        conn.transaction(|conn| {
            let mut db_new = DbNewPriceLevel::from(new_price_level);

            let existing_count = price_levels::table
                .filter(price_levels::hub_id.eq(new_price_level.hub_id))
                .count()
                .get_result::<i64>(conn)?;

            if existing_count == 0 {
                db_new.is_default = true;
            }

            let created = diesel::insert_into(price_levels::table)
                .values(&db_new)
                .get_result::<DbPriceLevel>(conn)?;

            Ok::<DomainPriceLevel, diesel::result::Error>(created.into())
        })
        .map_err(Into::into)
    }

    fn update_price_level(
        &self,
        price_level_id: i32,
        hub_id: i32,
        updates: &DomainUpdatePriceLevel,
    ) -> RepositoryResult<DomainPriceLevel> {
        use crate::schema::price_levels;

        let mut conn = self.conn()?;
        let db_updates = DbUpdatePriceLevel::from(updates);

        let target = price_levels::table
            .filter(price_levels::id.eq(price_level_id))
            .filter(price_levels::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbPriceLevel>(&mut conn)?;

        Ok(updated.into())
    }

    fn delete_price_level(&self, price_level_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::price_levels;

        let mut conn = self.conn()?;

        let target = price_levels::table
            .filter(price_levels::id.eq(price_level_id))
            .filter(price_levels::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
