use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::order::{
    NewOrder as DomainNewOrder, Order as DomainOrder, OrderProduct as DomainOrderProduct,
    UpdateOrder as DomainUpdateOrder,
};
use crate::domain::types::{
    CurrencyCode, CustomerId, HubId, OrderId, OrderNotes, OrderReference, PriceCents,
    ProductDescription, ProductId, ProductName, ProductQuantity, ProductSku,
};

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
#[diesel(treat_none_as_null = true)]
pub struct UpdateOrder<'a> {
    pub status: &'a str,
    pub notes: Option<&'a str>,
    pub total_cents: i32,
    pub currency: &'a str,
    pub customer_id: Option<i32>,
    pub reference: Option<&'a str>,
    pub updated_at: NaiveDateTime,
}

impl Order {
    pub fn into_domain(self, products: Vec<OrderProduct>) -> DomainOrder {
        DomainOrder {
            id: OrderId::new(self.id).expect("valid order id from database"),
            hub_id: HubId::new(self.hub_id).expect("valid hub id from database"),
            customer_id: self
                .customer_id
                .map(|id| CustomerId::new(id).expect("valid customer id from database")),
            reference: self.reference.and_then(|r| OrderReference::new(r).ok()),
            status: self.status.as_str().into(),
            notes: self.notes.and_then(|n| OrderNotes::new(n).ok()),
            total_cents: PriceCents::new(self.total_cents).expect("valid price from database"),
            currency: CurrencyCode::new(self.currency).expect("valid currency from database"),
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
            product_id: self
                .product_id
                .map(|id| ProductId::new(id).expect("valid product id from database")),
            name: ProductName::new(self.name).expect("valid product name from database"),
            sku: self.sku.and_then(|s| ProductSku::new(s).ok()),
            description: self
                .description
                .and_then(|d| ProductDescription::new(d).ok()),
            price_cents: PriceCents::new(self.price_cents).expect("valid price from database"),
            currency: CurrencyCode::new(self.currency).expect("valid currency from database"),
            quantity: ProductQuantity::new(self.quantity).expect("valid quantity from database"),
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
        }
    }
}

impl<'a> From<&'a DomainUpdateOrder> for UpdateOrder<'a> {
    fn from(value: &'a DomainUpdateOrder) -> Self {
        Self {
            status: value.status.into(),
            notes: value.notes.as_ref().map(|n| n.as_str()),
            total_cents: value.total_cents.get(),
            currency: value.currency.as_str(),
            customer_id: value.customer_id.map(|id| id.get()),
            reference: value.reference.as_ref().map(|r| r.as_str()),
            updated_at: value.updated_at,
        }
    }
}
