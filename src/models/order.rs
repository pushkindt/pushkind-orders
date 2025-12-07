//! Diesel models for order and order product records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::order::{
    NewOrder as DomainNewOrder, Order as DomainOrder, OrderProduct as DomainOrderProduct,
    UpdateOrder as DomainUpdateOrder,
};
use crate::domain::types::{
    CurrencyCode, CustomerId, HubId, OrderConsignee, OrderDeliveryNotes, OrderId, OrderNotes,
    OrderPayer, OrderReference, OrderShippingAddress, PriceCents, ProductDescription, ProductId,
    ProductName, ProductQuantity, ProductSku, TypeConstraintError,
};

/// Database representation of an order record.
#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(
    table_name = crate::schema::orders,
    belongs_to(super::customer::Customer, foreign_key = customer_id)
)]
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
    pub shipping_address: Option<String>,
    pub consignee: Option<String>,
    pub delivery_notes: Option<String>,
    pub payer: Option<String>,
}

/// Database representation of a product snapshot within an order.
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
    pub default_price_cents: Option<i32>,
}

/// Payload for inserting a new order record.
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
}

/// Payload for inserting a new order product record.
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
    pub default_price_cents: Option<i32>,
}

/// Payload for updating an existing order record.
#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::orders)]
#[diesel(treat_none_as_null = true)]
pub struct UpdateOrder<'a> {
    pub status: &'a str,
    pub notes: Option<&'a str>,
    pub reference: Option<&'a str>,
    pub updated_at: NaiveDateTime,
    pub shipping_address: Option<&'a str>,
    pub consignee: Option<&'a str>,
    pub delivery_notes: Option<&'a str>,
    pub payer: Option<&'a str>,
}

impl TryFrom<(Order, Vec<OrderProduct>)> for DomainOrder {
    type Error = TypeConstraintError;
    fn try_from(value: (Order, Vec<OrderProduct>)) -> Result<Self, Self::Error> {
        let (order, products) = value;
        Ok(Self {
            id: OrderId::new(order.id)?,
            hub_id: HubId::new(order.hub_id)?,
            customer_id: order.customer_id.map(CustomerId::new).transpose()?,
            reference: order.reference.and_then(|r| OrderReference::new(r).ok()),
            status: order.status.as_str().into(),
            notes: order.notes.and_then(|n| OrderNotes::new(n).ok()),
            total_cents: PriceCents::new(order.total_cents)?,
            currency: CurrencyCode::new(order.currency)?,
            products: products
                .iter()
                .map(|p| p.try_into())
                .collect::<Result<Vec<DomainOrderProduct>, Self::Error>>()?,
            created_at: order.created_at,
            updated_at: order.updated_at,
            shipping_address: order
                .shipping_address
                .and_then(|s| OrderShippingAddress::new(s).ok()),
            consignee: order.consignee.and_then(|c| OrderConsignee::new(c).ok()),
            delivery_notes: order
                .delivery_notes
                .and_then(|d| OrderDeliveryNotes::new(d).ok()),
            payer: order.payer.and_then(|p| OrderPayer::new(p).ok()),
        })
    }
}

impl TryFrom<&OrderProduct> for DomainOrderProduct {
    type Error = TypeConstraintError;
    fn try_from(value: &OrderProduct) -> Result<Self, Self::Error> {
        Ok(Self {
            product_id: value.product_id.map(ProductId::new).transpose()?,
            name: ProductName::new(value.name.clone())?,
            sku: value.sku.clone().and_then(|s| ProductSku::new(s).ok()),
            description: value
                .description
                .clone()
                .and_then(|d| ProductDescription::new(d).ok()),
            price_cents: PriceCents::new(value.price_cents)?,
            currency: CurrencyCode::new(value.currency.clone())?,
            quantity: ProductQuantity::new(value.quantity)?,
            default_price_cents: value.default_price_cents.map(PriceCents::new).transpose()?,
        })
    }
}

impl<'a> From<&'a DomainNewOrder> for NewOrder<'a> {
    fn from(value: &'a DomainNewOrder) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            customer_id: value.customer_id.map(|id| id.get()),
            reference: value.reference.as_ref().map(|r| r.as_str()),
            status: value.status.into(),
            notes: value.notes.as_ref().map(|n| n.as_str()),
            total_cents: value.total_cents.get(),
            currency: value.currency.as_str(),
        }
    }
}

impl<'a> NewOrderProduct<'a> {
    pub fn from_domain(order_id: i32, value: &'a DomainOrderProduct) -> Self {
        Self {
            order_id,
            product_id: value.product_id.map(|id| id.get()),
            name: value.name.as_str(),
            sku: value.sku.as_ref().map(|s| s.as_str()),
            description: value.description.as_ref().map(|d| d.as_str()),
            price_cents: value.price_cents.get(),
            currency: value.currency.as_str(),
            quantity: value.quantity.get(),
            default_price_cents: value.default_price_cents.map(|cents| cents.get()),
        }
    }
}

impl<'a> From<&'a DomainUpdateOrder> for UpdateOrder<'a> {
    fn from(value: &'a DomainUpdateOrder) -> Self {
        Self {
            status: value.status.into(),
            notes: value.notes.as_ref().map(|n| n.as_str()),
            reference: value.reference.as_ref().map(|r| r.as_str()),
            updated_at: value.updated_at,
            shipping_address: value.shipping_address.as_ref().map(|s| s.as_str()),
            consignee: value.consignee.as_ref().map(|c| c.as_str()),
            delivery_notes: value.delivery_notes.as_ref().map(|d| d.as_str()),
            payer: value.payer.as_ref().map(|p| p.as_str()),
        }
    }
}
