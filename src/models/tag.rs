use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::tag::{NewTag as DomainNewTag, Tag as DomainTag, UpdateTag as DomainUpdateTag};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::tags)]
pub struct Tag {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tags)]
pub struct NewTag<'a> {
    pub hub_id: i32,
    pub name: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::tags)]
pub struct UpdateTag<'a> {
    pub name: &'a str,
    pub updated_at: NaiveDateTime,
}

impl From<Tag> for DomainTag {
    fn from(value: Tag) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            name: value.name,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainNewTag> for NewTag<'a> {
    fn from(value: &'a DomainNewTag) -> Self {
        Self {
            hub_id: value.hub_id,
            name: value.name.as_str(),
        }
    }
}

impl<'a> From<&'a DomainUpdateTag> for UpdateTag<'a> {
    fn from(value: &'a DomainUpdateTag) -> Self {
        Self {
            name: value.name.as_str(),
            updated_at: value.updated_at,
        }
    }
}
