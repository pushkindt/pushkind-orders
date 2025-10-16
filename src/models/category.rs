use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::category::{Category as DomainCategory, NewCategory as DomainNewCategory};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::categories)]
pub struct Category {
    pub id: i32,
    pub hub_id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub is_archived: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::categories)]
pub struct NewCategory<'a> {
    pub hub_id: i32,
    pub parent_id: Option<i32>,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub updated_at: NaiveDateTime,
}

impl From<Category> for DomainCategory {
    fn from(value: Category) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            parent_id: value.parent_id,
            name: value.name,
            description: value.description,
            is_archived: value.is_archived,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainNewCategory> for NewCategory<'a> {
    fn from(value: &'a DomainNewCategory) -> Self {
        Self {
            hub_id: value.hub_id,
            parent_id: value.parent_id,
            name: value.name.as_str(),
            description: value.description.as_deref(),
            updated_at: value.updated_at,
        }
    }
}
