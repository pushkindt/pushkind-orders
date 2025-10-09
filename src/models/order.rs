use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::order::{
    NewOrder as DomainNewOrder, Order as DomainOrder, OrderProduct as DomainOrderProduct,
    UpdateOrder as DomainUpdateOrder,
};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::orders)]
pub struct Order {
    pub id: i32,
    pub hub_id: i32,
    pub customer_id: Option<i32>,
    pub reference: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub total_cents: i32,
    pub currency: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(table_name = crate::schema::order_products)]
#[diesel(belongs_to(Order, foreign_key = order_id))]
pub struct OrderProduct {
    pub id: i32,
    pub order_id: i32,
    pub product_id: Option<i32>,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub price_cents: i32,
    pub currency: String,
    pub quantity: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::orders)]
pub struct NewOrder<'a> {
    pub hub_id: i32,
    pub customer_id: Option<i32>,
    pub reference: Option<&'a str>,
    pub status: &'a str,
    pub notes: Option<&'a str>,
    pub total_cents: i32,
    pub currency: &'a str,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::order_products)]
pub struct NewOrderProduct<'a> {
    pub order_id: i32,
    pub product_id: Option<i32>,
    pub name: &'a str,
    pub sku: Option<&'a str>,
    pub description: Option<&'a str>,
    pub price_cents: i32,
    pub currency: &'a str,
    pub quantity: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::orders)]
pub struct UpdateOrder<'a> {
    pub status: Option<&'a str>,
    pub notes: Option<Option<&'a str>>,
    pub total_cents: Option<i32>,
    pub currency: Option<&'a str>,
    pub customer_id: Option<Option<i32>>,
    pub reference: Option<Option<&'a str>>,
    pub updated_at: NaiveDateTime,
}

impl Order {
    pub fn into_domain(self, products: Vec<OrderProduct>) -> DomainOrder {
        DomainOrder {
            id: self.id,
            hub_id: self.hub_id,
            customer_id: self.customer_id,
            reference: self.reference,
            status: self.status.as_str().into(),
            notes: self.notes,
            total_cents: self.total_cents,
            currency: self.currency,
            products: products
                .into_iter()
                .map(OrderProduct::into_domain)
                .collect(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl OrderProduct {
    pub fn into_domain(self) -> DomainOrderProduct {
        DomainOrderProduct {
            product_id: self.product_id,
            name: self.name,
            sku: self.sku,
            description: self.description,
            price_cents: self.price_cents,
            currency: self.currency,
            quantity: self.quantity,
        }
    }
}

impl From<(Order, Vec<OrderProduct>)> for DomainOrder {
    fn from(value: (Order, Vec<OrderProduct>)) -> Self {
        value.0.into_domain(value.1)
    }
}

impl<'a> From<&'a DomainNewOrder> for NewOrder<'a> {
    fn from(value: &'a DomainNewOrder) -> Self {
        Self {
            hub_id: value.hub_id,
            customer_id: value.customer_id,
            reference: value.reference.as_deref(),
            status: value.status.into(),
            notes: value.notes.as_deref(),
            total_cents: value.total_cents,
            currency: value.currency.as_str(),
            updated_at: value.updated_at,
        }
    }
}

impl<'a> NewOrderProduct<'a> {
    pub fn from_domain(order_id: i32, value: &'a DomainOrderProduct) -> Self {
        Self {
            order_id,
            product_id: value.product_id,
            name: value.name.as_str(),
            sku: value.sku.as_deref(),
            description: value.description.as_deref(),
            price_cents: value.price_cents,
            currency: value.currency.as_str(),
            quantity: value.quantity,
        }
    }
}

impl<'a> From<&'a DomainUpdateOrder> for UpdateOrder<'a> {
    fn from(value: &'a DomainUpdateOrder) -> Self {
        Self {
            status: value.status.map(|status| status.into()),
            notes: value
                .notes
                .as_ref()
                .map(|notes| notes.as_ref().map(String::as_str)),
            total_cents: value.total_cents,
            currency: value.currency.as_deref(),
            customer_id: value.customer_id,
            reference: value
                .reference
                .as_ref()
                .map(|reference| reference.as_ref().map(String::as_str)),
            updated_at: value.updated_at,
        }
    }
}
