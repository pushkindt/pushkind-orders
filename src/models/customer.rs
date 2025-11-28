//! Diesel model for customer records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    customer::{Customer as DomainCustomer, NewCustomer as DomainNewCustomer},
    types::{
        CustomerId, CustomerName, HubId, PhoneNumber, PriceLevelId, TypeConstraintError, UserEmail,
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
    pub email: Option<String>,
}

/// Payload for inserting a new customer record.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::customers)]
pub struct NewCustomer<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub phone: &'a str,
    pub price_level_id: Option<i32>,
}

impl TryFrom<Customer> for DomainCustomer {
    type Error = TypeConstraintError;

    fn try_from(value: Customer) -> Result<Self, Self::Error> {
        Ok(Self {
            id: CustomerId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: CustomerName::new(value.name)?,
            email: value.email.map(UserEmail::new).transpose()?,
            phone: PhoneNumber::new(value.phone)?,
            price_level_id: value.price_level_id.map(PriceLevelId::new).transpose()?,
        })
    }
}

impl<'a> From<&'a DomainNewCustomer> for NewCustomer<'a> {
    fn from(value: &'a DomainNewCustomer) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            name: value.name.as_str(),
            email: value.email.as_ref().map(|email| email.as_str()),
            phone: value.phone.as_str(),
            price_level_id: value.price_level_id.map(|id| id.get()),
        }
    }
}
