use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::tag::{
    NewTag as DomainNewTag, Tag as DomainTag, TagListQuery, UpdateTag as DomainUpdateTag,
};
use crate::models::tag::{NewTag as DbNewTag, Tag as DbTag, UpdateTag as DbUpdateTag};
use crate::repository::{DieselRepository, TagReader, TagWriter};

impl TagReader for DieselRepository {
    fn list_tags(&self, query: TagListQuery) -> RepositoryResult<(usize, Vec<DomainTag>)> {
        use crate::schema::tags;

        let mut conn = self.conn()?;

        let mut count_query = tags::table
            .filter(tags::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(search) = query.search.as_ref() {
            let pattern = format!("%{}%", search);
            count_query = count_query.filter(tags::name.like(pattern.clone()));
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items_query = tags::table
            .filter(tags::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(search) = query.search.as_ref() {
            let pattern = format!("%{}%", search);
            items_query = items_query.filter(tags::name.like(pattern));
        }

        items_query = items_query.order(tags::name.asc());

        if let Some(pagination) = &query.pagination {
            let page = pagination.page.max(1);
            let per_page = pagination.per_page as i64;
            let offset = ((page - 1) * pagination.per_page) as i64;
            items_query = items_query.offset(offset).limit(per_page);
        }

        let db_tags = items_query.load::<DbTag>(&mut conn)?;
        let tags = db_tags.into_iter().map(DomainTag::from).collect();

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

        Ok(created.into())
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

        Ok(updated.into())
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
