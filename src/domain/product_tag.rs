use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{ProductId, ProductTagId, TagId, TypeConstraintError};

/// Domain representation linking a product to a tag record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProductTag {
    /// Unique identifier of the product-tag association.
    pub id: ProductTagId,
    /// Identifier of the product the tag is attached to.
    pub product_id: ProductId,
    /// Identifier of the referenced tag record.
    pub tag_id: TagId,
    /// Timestamp for when the association was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the association.
    pub updated_at: NaiveDateTime,
}

/// Payload required to associate an existing tag with a product.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NewProductTag {
    /// Identifier of the product receiving the tag.
    pub product_id: ProductId,
    /// Identifier of the tag being attached to the product.
    pub tag_id: TagId,
}

impl NewProductTag {
    /// Construct a new association payload between a product and a tag.
    pub fn new(product_id: ProductId, tag_id: TagId) -> Self {
        Self { product_id, tag_id }
    }

    /// Attempt to construct an association by validating identifiers.
    pub fn try_new(product_id: i32, tag_id: i32) -> Result<Self, TypeConstraintError> {
        let tag_id = TagId::new(tag_id)?;
        Ok(Self::new(ProductId::new(product_id)?, tag_id))
    }
}
