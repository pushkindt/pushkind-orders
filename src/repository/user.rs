use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::user::{NewUser as DomainNewUser, UpdateUser as DomainUpdateUser, User as DomainUser},
    models::user::{NewUser as DbNewUser, UpdateUser as DbUpdateUser, User as DbUser},
    repository::{DieselRepository, UserListQuery, UserReader, UserWriter},
};

impl UserReader for DieselRepository {
    fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainUser>> {
        use crate::schema::users;

        let mut conn = self.conn()?;
        let user = users::table
            .filter(users::id.eq(Some(id)))
            .filter(users::hub_id.eq(hub_id))
            .first::<DbUser>(&mut conn)
            .optional()?;

        Ok(user.map(Into::into))
    }

    fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<DomainUser>> {
        use crate::schema::users;

        let mut conn = self.conn()?;
        let user = users::table
            .filter(users::email.eq(email))
            .filter(users::hub_id.eq(hub_id))
            .first::<DbUser>(&mut conn)
            .optional()?;

        Ok(user.map(Into::into))
    }

    fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<DomainUser>)> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let mut count_query = users::table
            .filter(users::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            count_query = count_query.filter(
                users::name
                    .like(pattern.clone())
                    .or(users::email.like(pattern)),
            );
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items = users::table
            .filter(users::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            items = items.filter(
                users::name
                    .like(pattern.clone())
                    .or(users::email.like(pattern)),
            );
        }

        items = items.order(users::created_at.desc());

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_users = items.load::<DbUser>(&mut conn)?;

        if db_users.is_empty() {
            return Ok((total, Vec::new()));
        }

        Ok((total, db_users.into_iter().map(Into::into).collect()))
    }
}

impl UserWriter for DieselRepository {
    fn create_user(&self, new_user: &DomainNewUser) -> RepositoryResult<DomainUser> {
        use crate::schema::users;

        let mut conn = self.conn()?;
        let db_new = DbNewUser::from(new_user);

        let created = diesel::insert_into(users::table)
            .values(&db_new)
            .get_result::<DbUser>(&mut conn)?;

        Ok(created.into())
    }

    fn update_user(
        &self,
        user_id: i32,
        hub_id: i32,
        updates: &DomainUpdateUser,
    ) -> RepositoryResult<DomainUser> {
        use crate::schema::users;

        let mut conn = self.conn()?;
        let db_updates = DbUpdateUser::from(updates);

        let target = users::table
            .filter(users::id.eq(Some(user_id)))
            .filter(users::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbUser>(&mut conn)?;

        Ok(updated.into())
    }

    fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let target = users::table
            .filter(users::id.eq(Some(user_id)))
            .filter(users::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
