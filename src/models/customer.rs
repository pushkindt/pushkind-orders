//! Diesel model for customer records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    customer::{
        Customer as DomainCustomer, NewCustomer as DomainNewCustomer,
        UpdateCustomer as DomainUpdateCustomer,
    },
    types::{
        CustomerId, CustomerName, HubId, PhoneNumber, PriceLevelId, PublicId, TypeConstraintError,
    },
};

/// Database representation of a customer record.
#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(
    table_name = crate::schema::customers,
    belongs_to(super::price_level::PriceLevel, foreign_key = price_level_id)
)]
pub struct Customer {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub price_level_id: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub phone: String,
    pub public_id: Option<String>,
}

/// Payload for inserting a new customer record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::customers)]
pub struct NewCustomer<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub public_id: Option<&'a str>,
    pub phone: &'a str,
    pub price_level_id: Option<i32>,
}

/// Payload for updating a customer record.
#[derive(AsChangeset)]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = crate::schema::customers)]
pub struct UpdateCustomer<'a> {
    pub name: &'a str,
    pub public_id: Option<&'a str>,
    pub price_level_id: Option<i32>,
}

impl TryFrom<Customer> for DomainCustomer {
    type Error = TypeConstraintError;

    fn try_from(value: Customer) -> Result<Self, Self::Error> {
        Ok(Self {
            id: CustomerId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: CustomerName::new(value.name)?,
            phone: PhoneNumber::new(value.phone)?,
            price_level_id: value.price_level_id.map(PriceLevelId::new).transpose()?,
            public_id: value.public_id.map(PublicId::new).transpose()?,
        })
    }
}

impl<'a> From<&'a DomainNewCustomer> for NewCustomer<'a> {
    fn from(value: &'a DomainNewCustomer) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            name: value.name.as_str(),
            phone: value.phone.as_str(),
            price_level_id: value.price_level_id.map(|id| id.get()),
            public_id: value.public_id.as_ref().map(|id| id.as_str()),
        }
    }
}

impl<'a> From<&'a DomainUpdateCustomer> for UpdateCustomer<'a> {
    fn from(value: &'a DomainUpdateCustomer) -> Self {
        Self {
            name: value.name.as_str(),
            price_level_id: value.price_level_id.map(|id| id.get()),
            public_id: value.public_id.as_ref().map(|id| id.as_str()),
        }
    }
}
