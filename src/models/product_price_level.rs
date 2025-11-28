//! Diesel model for product price level association records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::product_price_level::{
    NewProductPriceLevelRate as DomainNewProductPriceLevelRate,
    ProductPriceLevelRate as DomainProductPriceLevelRate,
    UpdateProductPriceLevelRate as DomainUpdateProductPriceLevelRate,
};
use crate::domain::types::{PriceLevelId, ProductId, ProductPriceLevelRateId, TypeConstraintError};

#[derive(Debug, Clone, Identifiable, Queryable, Associations, Selectable)]
#[diesel(
    table_name = crate::schema::product_price_levels,
    belongs_to(super::product::Product, foreign_key = product_id),
    belongs_to(super::price_level::PriceLevel, foreign_key = price_level_id)
)]
pub struct ProductPriceLevel {
    pub id: i32,
    pub product_id: i32,
    pub price_level_id: i32,
    pub price_cents: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::product_price_levels)]
pub struct NewProductPriceLevel {
    pub product_id: i32,
    pub price_level_id: i32,
    pub price_cents: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::product_price_levels)]
#[diesel(treat_none_as_null = true)]
pub struct UpdateProductPriceLevel {
    pub price_cents: i32,
    pub updated_at: NaiveDateTime,
}

impl TryFrom<ProductPriceLevel> for DomainProductPriceLevelRate {
    type Error = TypeConstraintError;

    fn try_from(value: ProductPriceLevel) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ProductPriceLevelRateId::new(value.id)?,
            product_id: ProductId::new(value.product_id)?,
            price_level_id: PriceLevelId::new(value.price_level_id)?,
            price_cents: value.price_cents.try_into()?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

impl From<&DomainNewProductPriceLevelRate> for NewProductPriceLevel {
    fn from(value: &DomainNewProductPriceLevelRate) -> Self {
        Self {
            product_id: value.product_id.get(),
            price_level_id: value.price_level_id.get(),
            price_cents: value.price_cents.get(),
        }
    }
}

impl From<&DomainUpdateProductPriceLevelRate> for UpdateProductPriceLevel {
    fn from(value: &DomainUpdateProductPriceLevelRate) -> Self {
        Self {
            price_cents: value.price_cents,
            updated_at: value.updated_at,
        }
    }
}
