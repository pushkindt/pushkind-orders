//! Tag repository implementation with Diesel.

use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::tag::{
    NewTag as DomainNewTag, Tag as DomainTag, TagListQuery, UpdateTag as DomainUpdateTag,
};
use crate::domain::types::TypeConstraintError;
use crate::models::tag::{NewTag as DbNewTag, Tag as DbTag, UpdateTag as DbUpdateTag};
use crate::repository::{DieselRepository, TagReader, TagWriter};

fn map_type_error(err: TypeConstraintError) -> RepositoryError {
    RepositoryError::Unexpected(format!("Invalid tag data: {err}"))
}

impl TagReader for DieselRepository {
    fn list_tags(&self, query: TagListQuery) -> RepositoryResult<(usize, Vec<DomainTag>)> {
        use crate::schema::tags;

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = tags::table
                .filter(tags::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(search) = query.search.as_ref() {
                let pattern = format!("%{}%", search);
                items = items.filter(tags::name.like(pattern.clone()));
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items_query = query_builder().order(tags::name.asc());

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items_query = items_query.offset(offset).limit(limit);
        }

        let db_tags = items_query.load::<DbTag>(&mut conn)?;
        let tags = db_tags
            .into_iter()
            .map(DomainTag::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_type_error)?;

        Ok((total, tags))
    }
}

impl TagWriter for DieselRepository {
    fn create_tag(&self, new_tag: &DomainNewTag) -> RepositoryResult<DomainTag> {
        use crate::schema::tags;

        let mut conn = self.conn()?;
        let insertable = DbNewTag::from(new_tag);

        let created = diesel::insert_into(tags::table)
            .values(&insertable)
            .get_result::<DbTag>(&mut conn)?;

        DomainTag::try_from(created).map_err(map_type_error)
    }

    fn update_tag(
        &self,
        tag_id: i32,
        hub_id: i32,
        updates: &DomainUpdateTag,
    ) -> RepositoryResult<DomainTag> {
        use crate::schema::tags;

        let mut conn = self.conn()?;
        let db_updates = DbUpdateTag::from(updates);

        let target = tags::table
            .filter(tags::id.eq(tag_id))
            .filter(tags::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbTag>(&mut conn)?;

        DomainTag::try_from(updated).map_err(map_type_error)
    }

    fn delete_tag(&self, tag_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::tags;

        let mut conn = self.conn()?;
        let target = tags::table
            .filter(tags::id.eq(tag_id))
            .filter(tags::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
