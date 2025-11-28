//! Diesel model for category records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::category::{
    Category as DomainCategory, NewCategory as DomainNewCategory,
    UpdateCategory as DomainUpdateCategory,
};
use crate::domain::types::{
    CategoryDescription, CategoryId, CategoryName, HubId, ImageUrl, TypeConstraintError,
};

/// Database representation of a category record.
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
    pub image_url: Option<String>,
}

/// Payload for inserting a new category record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::categories)]
pub struct NewCategory<'a> {
    pub hub_id: i32,
    pub parent_id: Option<i32>,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub image_url: Option<&'a str>,
}

/// Payload for updating an existing category record.
#[derive(Default, AsChangeset)]
#[diesel(table_name = crate::schema::categories, treat_none_as_null = true)]
pub struct UpdateCategory<'a> {
    pub name: &'a str,
    pub is_archived: bool,
    pub updated_at: NaiveDateTime,
    pub description: Option<&'a str>,
    pub image_url: Option<&'a str>,
}

impl TryFrom<Category> for DomainCategory {
    type Error = TypeConstraintError;

    fn try_from(value: Category) -> Result<Self, Self::Error> {
        Ok(Self {
            id: CategoryId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            parent_id: value.parent_id.map(CategoryId::new).transpose()?,
            name: CategoryName::new(value.name)?,
            description: value
                .description
                .map(CategoryDescription::new)
                .transpose()?,
            is_archived: value.is_archived,
            created_at: value.created_at,
            updated_at: value.updated_at,
            image_url: value.image_url.map(ImageUrl::new).transpose()?,
        })
    }
}

impl<'a> From<&'a DomainNewCategory> for NewCategory<'a> {
    fn from(value: &'a DomainNewCategory) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            parent_id: value.parent_id.map(|id| id.get()),
            name: value.name.as_str(),
            description: value.description.as_ref().map(|value| value.as_str()),
            image_url: value.image_url.as_ref().map(|value| value.as_str()),
        }
    }
}

impl<'a> From<&'a DomainUpdateCategory> for UpdateCategory<'a> {
    fn from(value: &'a DomainUpdateCategory) -> Self {
        Self {
            is_archived: value.is_archived,
            updated_at: value.updated_at,
            name: value.name.as_str(),
            description: value.description.as_ref().map(|value| value.as_str()),
            image_url: value.image_url.as_ref().map(|value| value.as_str()),
        }
    }
}
