use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    PriceCents, PriceLevelId, ProductId, ProductPriceLevelRateId, TypeConstraintError,
};

/// Domain representation tying a product to a specific price level with an amount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductPriceLevelRate {
    /// Unique identifier of the association record.
    pub id: ProductPriceLevelRateId,
    /// Identifier of the product receiving this price.
    pub product_id: ProductId,
    /// Identifier of the price level applied to the product.
    pub price_level_id: PriceLevelId,
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: PriceCents,
    /// Timestamp for when the association record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the association record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new price level rate for a product.
#[derive(Debug, Clone, PartialEq)]
pub struct NewProductPriceLevelRate {
    /// Identifier of the product receiving this price.
    pub product_id: ProductId,
    /// Identifier of the price level applied to the product.
    pub price_level_id: PriceLevelId,
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: PriceCents,
}

impl NewProductPriceLevelRate {
    /// Construct a new association payload between a product and a price level.
    pub fn new(
        product_id: ProductId,
        price_level_id: PriceLevelId,
        price_cents: PriceCents,
    ) -> Self {
        Self {
            product_id,
            price_level_id,
            price_cents,
        }
    }

    /// Attempt to construct a new association from raw identifiers.
    pub fn try_new(
        product_id: i32,
        price_level_id: i32,
        price_cents: i32,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ProductId::new(product_id)?,
            PriceLevelId::new(price_level_id)?,
            PriceCents::new(price_cents)?,
        ))
    }
}

/// Patch data applied when updating an existing product price level rate.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateProductPriceLevelRate {
    /// Price update in the smallest currency unit.
    pub price_cents: i32,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}
