use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::price_level::{
    NewPriceLevel as DomainNewPriceLevel, PriceLevel as DomainPriceLevel,
    UpdatePriceLevel as DomainUpdatePriceLevel,
};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::price_levels)]
pub struct PriceLevel {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_default: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::price_levels)]
pub struct NewPriceLevel<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub is_default: bool,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::price_levels)]
#[diesel(treat_none_as_null = true)]
pub struct UpdatePriceLevel<'a> {
    pub name: &'a str,
    pub updated_at: NaiveDateTime,
    pub is_default: bool,
}

impl From<PriceLevel> for DomainPriceLevel {
    fn from(value: PriceLevel) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            name: value.name,
            created_at: value.created_at,
            updated_at: value.updated_at,
            is_default: value.is_default,
        }
    }
}

impl<'a> From<&'a DomainNewPriceLevel> for NewPriceLevel<'a> {
    fn from(value: &'a DomainNewPriceLevel) -> Self {
        Self {
            hub_id: value.hub_id,
            name: value.name.as_str(),
            is_default: value.is_default,
        }
    }
}

impl<'a> From<&'a DomainUpdatePriceLevel> for UpdatePriceLevel<'a> {
    fn from(value: &'a DomainUpdatePriceLevel) -> Self {
        Self {
            name: value.name.as_str(),
            updated_at: value.updated_at,
            is_default: value.is_default,
        }
    }
}
