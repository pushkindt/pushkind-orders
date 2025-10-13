use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::product::{
    NewProduct as DomainNewProduct, Product as DomainProduct, UpdateProduct as DomainUpdateProduct,
};

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
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::products)]
pub struct NewProduct<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub sku: Option<&'a str>,
    pub description: Option<&'a str>,
    pub currency: &'a str,
    pub updated_at: NaiveDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::products)]
pub struct UpdateProduct<'a> {
    pub name: Option<&'a str>,
    pub sku: Option<Option<&'a str>>,
    pub description: Option<Option<&'a str>>,
    pub currency: Option<&'a str>,
    pub is_archived: Option<bool>,
    pub updated_at: NaiveDateTime,
}

impl From<Product> for DomainProduct {
    fn from(value: Product) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            name: value.name,
            sku: value.sku,
            description: value.description,
            currency: value.currency,
            is_archived: value.is_archived,
            price_levels: Vec::new(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainNewProduct> for NewProduct<'a> {
    fn from(value: &'a DomainNewProduct) -> Self {
        Self {
            hub_id: value.hub_id,
            name: value.name.as_str(),
            sku: value.sku.as_deref(),
            description: value.description.as_deref(),
            currency: value.currency.as_str(),
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainUpdateProduct> for UpdateProduct<'a> {
    fn from(value: &'a DomainUpdateProduct) -> Self {
        Self {
            name: value.name.as_deref(),
            sku: value
                .sku
                .as_ref()
                .map(|sku| sku.as_ref().map(String::as_str)),
            description: value
                .description
                .as_ref()
                .map(|description| description.as_ref().map(String::as_str)),
            currency: value.currency.as_deref(),
            is_archived: value.is_archived,
            updated_at: value.updated_at,
        }
    }
}
