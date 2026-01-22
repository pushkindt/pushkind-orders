//! Diesel model for product records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    product::{
        NewProduct as DomainNewProduct, Product as DomainProduct,
        UpdateProduct as DomainUpdateProduct,
    },
    types::{
        CategoryId, CurrencyCode, HubId, ProductAmount, ProductDescription, ProductId, ProductName,
        ProductSku, ProductUnits, TypeConstraintError,
    },
};

/// Database representation of a product record.
#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::products)]
pub struct Product {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub currency: String,
    pub is_archived: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub units: Option<String>,
    pub category_id: Option<i32>,
    pub amount: Option<f32>,
    pub vendor_id: Option<i32>,
}

/// Payload for inserting a new product record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::products)]
pub struct NewProduct<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub sku: Option<&'a str>,
    pub description: Option<&'a str>,
    pub units: Option<&'a str>,
    pub currency: &'a str,
    pub category_id: Option<i32>,
    pub amount: Option<f32>,
    pub vendor_id: Option<i32>,
}

/// Payload for updating an existing product record.
#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::products)]
#[diesel(treat_none_as_null = true)]
pub struct UpdateProduct<'a> {
    pub name: &'a str,
    pub sku: Option<&'a str>,
    pub description: Option<&'a str>,
    pub units: Option<&'a str>,
    pub currency: &'a str,
    pub is_archived: bool,
    pub updated_at: NaiveDateTime,
    pub category_id: Option<i32>,
    pub amount: Option<f32>,
    pub vendor_id: Option<i32>,
}

impl TryFrom<Product> for DomainProduct {
    type Error = TypeConstraintError;

    fn try_from(value: Product) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ProductId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: ProductName::new(value.name)?,
            sku: value.sku.map(ProductSku::new).transpose()?,
            description: value.description.map(ProductDescription::new).transpose()?,
            units: value.units.map(ProductUnits::new).transpose()?,
            currency: CurrencyCode::new(value.currency)?,
            is_archived: value.is_archived,
            category_id: value.category_id.map(CategoryId::new).transpose()?,
            price_levels: Vec::new(),
            tags: Vec::new(),
            image_urls: Vec::new(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            amount: value.amount.map(ProductAmount::new).transpose()?,
        })
    }
}

impl<'a> From<&'a DomainNewProduct> for NewProduct<'a> {
    fn from(value: &'a DomainNewProduct) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            name: value.name.as_str(),
            sku: value.sku.as_ref().map(|sku| sku.as_str()),
            description: value.description.as_ref().map(|d| d.as_str()),
            units: value.units.as_ref().map(|units| units.as_str()),
            currency: value.currency.as_str(),
            category_id: value.category_id.map(|id| id.get()),
            amount: value.amount.map(|a| a.get()),
            vendor_id: None,
        }
    }
}

impl<'a> From<&'a DomainUpdateProduct> for UpdateProduct<'a> {
    fn from(value: &'a DomainUpdateProduct) -> Self {
        Self {
            name: value.name.as_str(),
            sku: value.sku.as_ref().map(|sku| sku.as_str()),
            description: value.description.as_ref().map(|d| d.as_str()),
            units: value.units.as_ref().map(|units| units.as_str()),
            currency: value.currency.as_str(),
            is_archived: value.is_archived,
            updated_at: value.updated_at,
            category_id: value.category_id.map(|id| id.get()),
            amount: value.amount.map(|a| a.get()),
            vendor_id: None,
        }
    }
}
