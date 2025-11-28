//! Category repository implementation with Diesel.

use diesel::dsl::{exists, select};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::category::{
    Category as DomainCategory, CategoryTreeQuery, NewCategory as DomainNewCategory,
    UpdateCategory as DomainUpdateCategory,
};
use crate::domain::types::TypeConstraintError;
use crate::models::category::{
    Category as DbCategory, NewCategory as DbNewCategory, UpdateCategory,
};
use crate::repository::{CategoryReader, CategoryWriter, DieselRepository};

fn map_type_error(err: TypeConstraintError) -> RepositoryError {
    RepositoryError::Unexpected(format!("Invalid category data: {err}"))
}

impl CategoryReader for DieselRepository {
    fn list_categories(
        &self,
        query: CategoryTreeQuery,
    ) -> RepositoryResult<(usize, Vec<DomainCategory>)> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = categories::table
                .filter(categories::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if !query.include_archived {
                items = items.filter(categories::is_archived.eq(false));
            }

            if let Some(term) = query.search.as_ref() {
                let pattern = format!("%{}%", term);
                items = items.filter(
                    categories::name
                        .like(pattern.clone())
                        .or(categories::description.like(pattern)),
                );
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items_query =
            query_builder().order((categories::parent_id.asc(), categories::name.asc()));

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items_query = items_query.offset(offset).limit(limit);
        }

        let categories = items_query.load::<DbCategory>(&mut conn)?;
        let categories = categories
            .into_iter()
            .map(DomainCategory::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_type_error)?;

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

        let category = category
            .map(DomainCategory::try_from)
            .transpose()
            .map_err(map_type_error)?;

        Ok(category)
    }

    fn get_category_by_name_and_parent(
        &self,
        name: &str,
        parent_id: Option<i32>,
        hub_id: i32,
    ) -> RepositoryResult<Option<DomainCategory>> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let mut query = categories::table
            .filter(categories::name.eq(name))
            .filter(categories::hub_id.eq(hub_id))
            .into_boxed();

        match parent_id {
            Some(id) => query = query.filter(categories::parent_id.eq(id)),
            None => query = query.filter(categories::parent_id.is_null()),
        }

        let category = query.first::<DbCategory>(&mut conn).optional()?;
        let category = category
            .map(DomainCategory::try_from)
            .transpose()
            .map_err(map_type_error)?;
        Ok(category)
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
            ensure_category_with_hub(&mut conn, new_category.hub_id.get(), parent_id.get())?;
        }

        let insertable = DbNewCategory::from(new_category);

        let created = diesel::insert_into(categories::table)
            .values(&insertable)
            .get_result::<DbCategory>(&mut conn)?;

        DomainCategory::try_from(created).map_err(map_type_error)
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

        DomainCategory::try_from(updated).map_err(map_type_error)
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
