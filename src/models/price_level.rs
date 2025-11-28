//! Diesel model for price level records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::price_level::{
    NewPriceLevel as DomainNewPriceLevel, PriceLevel as DomainPriceLevel,
    UpdatePriceLevel as DomainUpdatePriceLevel,
};
use crate::domain::types::{HubId, PriceLevelId, PriceLevelName, TypeConstraintError};

/// Database representation of a price level record.
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

/// Payload for inserting a new price level record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::price_levels)]
pub struct NewPriceLevel<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub is_default: bool,
}

/// Payload for updating an existing price level record.
#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::price_levels)]
#[diesel(treat_none_as_null = true)]
pub struct UpdatePriceLevel<'a> {
    pub name: &'a str,
    pub updated_at: NaiveDateTime,
    pub is_default: bool,
}

impl TryFrom<PriceLevel> for DomainPriceLevel {
    type Error = TypeConstraintError;

    fn try_from(value: PriceLevel) -> Result<Self, Self::Error> {
        Ok(Self {
            id: PriceLevelId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: PriceLevelName::new(value.name)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
            is_default: value.is_default,
        })
    }
}

impl<'a> From<&'a DomainNewPriceLevel> for NewPriceLevel<'a> {
    fn from(value: &'a DomainNewPriceLevel) -> Self {
        Self {
            hub_id: value.hub_id.get(),
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
