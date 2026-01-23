//! Diesel model for vendor order association records.

use diesel::prelude::*;

/// Database representation of a vendor-to-order association.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::vendor_order)]
pub struct VendorOrder {
    pub vendor_id: i32,
    pub order_id: i32,
}

/// Payload for inserting a new vendor-to-order association.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::vendor_order)]
pub struct NewVendorOrder {
    pub vendor_id: i32,
    pub order_id: i32,
}
