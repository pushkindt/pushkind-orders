use chrono::Local;
use diesel::dsl::{exists, select};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::category::{
    Category as DomainCategory, CategoryTreeQuery, NewCategory as DomainNewCategory,
    UpdateCategory as DomainUpdateCategory,
};
use crate::models::category::{
    Category as DbCategory, NewCategory as DbNewCategory, UpdateCategory,
};
use crate::repository::{CategoryReader, CategoryWriter, DieselRepository};

impl CategoryReader for DieselRepository {
    fn list_categories(
        &self,
        query: CategoryTreeQuery,
    ) -> RepositoryResult<(usize, Vec<DomainCategory>)> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let mut count_query = categories::table
            .filter(categories::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if !query.include_archived {
            count_query = count_query.filter(categories::is_archived.eq(false));
        }

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            count_query = count_query.filter(
                categories::name
                    .like(pattern.clone())
                    .or(categories::description.like(pattern)),
            );
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items_query = categories::table
            .filter(categories::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if !query.include_archived {
            items_query = items_query.filter(categories::is_archived.eq(false));
        }

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            items_query = items_query.filter(
                categories::name
                    .like(pattern.clone())
                    .or(categories::description.like(pattern)),
            );
        }

        items_query = items_query.order((categories::parent_id.asc(), categories::name.asc()));

        if let Some(pagination) = &query.pagination {
            let page = pagination.page.max(1);
            let per_page = pagination.per_page as i64;
            let offset = ((page - 1) * pagination.per_page) as i64;
            items_query = items_query.offset(offset).limit(per_page);
        }

        let categories = items_query.load::<DbCategory>(&mut conn)?;
        let categories = categories.into_iter().map(DomainCategory::from).collect();

        Ok((total, categories))
    }

    fn get_category_by_id(
        &self,
        category_id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<DomainCategory>> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let category = categories::table
            .filter(categories::id.eq(category_id))
            .filter(categories::hub_id.eq(hub_id))
            .first::<DbCategory>(&mut conn)
            .optional()?;

        Ok(category.map(DomainCategory::from))
    }
}

impl CategoryWriter for DieselRepository {
    fn create_category(
        &self,
        new_category: &DomainNewCategory,
    ) -> RepositoryResult<DomainCategory> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        if let Some(parent_id) = new_category.parent_id {
            ensure_category_with_hub(&mut conn, new_category.hub_id, parent_id)?;
        }

        let insertable = DbNewCategory::from(new_category);

        let created = diesel::insert_into(categories::table)
            .values(&insertable)
            .get_result::<DbCategory>(&mut conn)?;

        Ok(created.into())
    }

    fn update_category(
        &self,
        category_id: i32,
        hub_id: i32,
        updates: &DomainUpdateCategory,
    ) -> RepositoryResult<DomainCategory> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let db_updates = UpdateCategory::from(updates);

        let target = categories::table
            .filter(categories::id.eq(category_id))
            .filter(categories::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbCategory>(&mut conn)?;

        Ok(updated.into())
    }

    fn delete_category(&self, category_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let deleted = diesel::delete(
            categories::table
                .filter(categories::id.eq(category_id))
                .filter(categories::hub_id.eq(hub_id)),
        )
        .execute(&mut conn)?;

        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    fn assign_child_categories(
        &self,
        hub_id: i32,
        parent_id: i32,
        child_ids: &[i32],
    ) -> RepositoryResult<DomainCategory> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        conn.transaction::<DomainCategory, RepositoryError, _>(|conn| {
            ensure_category_with_hub(conn, hub_id, parent_id)?;

            let now = Local::now().naive_utc();

            diesel::update(
                categories::table
                    .filter(categories::hub_id.eq(hub_id))
                    .filter(categories::parent_id.eq(Some(parent_id))),
            )
            .set((
                categories::parent_id.eq::<Option<i32>>(None),
                categories::updated_at.eq(now),
            ))
            .execute(conn)?;

            if !child_ids.is_empty() {
                let valid_children = categories::table
                    .filter(categories::hub_id.eq(hub_id))
                    .filter(categories::id.eq_any(child_ids))
                    .select(categories::id)
                    .load::<i32>(conn)?;

                if valid_children.len() != child_ids.len() {
                    return Err(RepositoryError::NotFound);
                }

                diesel::update(
                    categories::table
                        .filter(categories::hub_id.eq(hub_id))
                        .filter(categories::id.eq_any(child_ids)),
                )
                .set((
                    categories::parent_id.eq(Some(parent_id)),
                    categories::updated_at.eq(now),
                ))
                .execute(conn)?;
            }

            diesel::update(
                categories::table
                    .filter(categories::id.eq(parent_id))
                    .filter(categories::hub_id.eq(hub_id)),
            )
            .set(categories::updated_at.eq(now))
            .execute(conn)?;

            let parent = categories::table
                .filter(categories::id.eq(parent_id))
                .filter(categories::hub_id.eq(hub_id))
                .first::<DbCategory>(conn)?;

            Ok(parent.into())
        })
    }
}

fn ensure_category_with_hub(
    conn: &mut SqliteConnection,
    hub_id: i32,
    category_id: i32,
) -> RepositoryResult<()> {
    use crate::schema::categories;

    let exists = select(exists(
        categories::table
            .filter(categories::id.eq(category_id))
            .filter(categories::hub_id.eq(hub_id)),
    ))
    .get_result(conn)?;

    if exists {
        Ok(())
    } else {
        Err(RepositoryError::NotFound)
    }
}
