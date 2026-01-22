use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    types::{HubId, TypeConstraintError, VendorId, VendorName},
    vendor::{
        NewVendor as DomainNewVendor, UpdateVendor as DomainUpdateVendor, Vendor as DomainVendor,
    },
};

/// Database representation of a vendor record.
#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::vendors)]
pub struct Vendor {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Payload for inserting a new vendor record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::vendors)]
pub struct NewVendor<'a> {
    pub hub_id: i32,
    pub name: &'a str,
}

/// Payload for updating an existing vendor record.
#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::vendors)]
pub struct UpdateVendor<'a> {
    pub name: &'a str,
    pub updated_at: NaiveDateTime,
}

impl TryFrom<Vendor> for DomainVendor {
    type Error = TypeConstraintError;

    fn try_from(value: Vendor) -> Result<Self, Self::Error> {
        Ok(Self {
            id: VendorId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: VendorName::new(value.name)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

impl<'a> From<&'a DomainNewVendor> for NewVendor<'a> {
    fn from(value: &'a DomainNewVendor) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            name: value.name.as_str(),
        }
    }
}

impl<'a> From<&'a DomainUpdateVendor> for UpdateVendor<'a> {
    fn from(value: &'a DomainUpdateVendor) -> Self {
        Self {
            name: value.name.as_str(),
            updated_at: value.updated_at,
        }
    }
}
