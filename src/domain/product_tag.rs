use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Domain representation linking a product to a tag record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProductTag {
    /// Unique identifier of the product-tag association.
    pub id: i32,
    /// Identifier of the product the tag is attached to.
    pub product_id: i32,
    /// Identifier of the referenced tag record.
    pub tag_id: i32,
    /// Timestamp for when the association was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the association.
    pub updated_at: NaiveDateTime,
}

/// Payload required to associate an existing tag with a product.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NewProductTag {
    /// Identifier of the product receiving the tag.
    pub product_id: i32,
    /// Identifier of the tag being attached to the product.
    pub tag_id: i32,
}

impl NewProductTag {
    /// Construct a new association payload between a product and a tag.
    pub fn new(product_id: i32, tag_id: i32) -> Self {
        Self { product_id, tag_id }
    }
}
