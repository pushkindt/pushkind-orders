use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Domain representation tying a product to a specific price level with an amount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductPriceLevelRate {
    /// Unique identifier of the association record.
    pub id: i32,
    /// Identifier of the product receiving this price.
    pub product_id: i32,
    /// Identifier of the price level applied to the product.
    pub price_level_id: i32,
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: i32,
    /// Timestamp for when the association record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the association record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new price level rate for a product.
#[derive(Debug, Clone, PartialEq)]
pub struct NewProductPriceLevelRate {
    /// Identifier of the product receiving this price.
    pub product_id: i32,
    /// Identifier of the price level applied to the product.
    pub price_level_id: i32,
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: i32,
}

impl NewProductPriceLevelRate {
    /// Construct a new association payload between a product and a price level.
    pub fn new(product_id: i32, price_level_id: i32, price_cents: i32) -> Self {
        Self {
            product_id,
            price_level_id,
            price_cents,
        }
    }
}

/// Patch data applied when updating an existing product price level rate.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateProductPriceLevelRate {
    /// Optional price update in the smallest currency unit.
    pub price_cents: Option<i32>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl Default for UpdateProductPriceLevelRate {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateProductPriceLevelRate {
    /// Create a new patch object with no changes applied yet.
    pub fn new() -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            price_cents: None,
            updated_at: now,
        }
    }

    /// Update the price stored for this association.
    pub fn price_cents(mut self, price_cents: i32) -> Self {
        self.price_cents = Some(price_cents);
        self
    }
}
