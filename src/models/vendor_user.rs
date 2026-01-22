//! Diesel model for vendor user association records.

use diesel::prelude::*;

/// Database representation of a vendor-to-user association.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::vendor_user)]
pub struct VendorUser {
    pub vendor_id: i32,
    pub user_id: i32,
}

/// Payload for inserting a new vendor-to-user association.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::vendor_user)]
pub struct NewVendorUser {
    pub vendor_id: i32,
    pub user_id: i32,
}
