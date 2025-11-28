//! Diesel model for product image records.

use diesel::prelude::*;

/// Database representation of a product image record.
#[derive(Debug, Clone, Identifiable, Queryable, Associations, Selectable)]
#[diesel(
    table_name = crate::schema::product_images,
    belongs_to(super::product::Product, foreign_key = product_id)
)]
pub struct ProductImage {
    pub id: i32,
    pub product_id: i32,
    pub image_url: String,
}

/// Payload for inserting a new product image record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::product_images)]
pub struct NewProductImage<'a> {
    pub product_id: i32,
    pub image_url: &'a str,
}
