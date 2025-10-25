use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::customer::{Customer as DomainCustomer, NewCustomer as DomainNewCustomer};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(
    table_name = crate::schema::customers,
    belongs_to(super::price_level::PriceLevel, foreign_key = price_level_id)
)]
pub struct Customer {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
    pub price_level_id: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::customers)]
pub struct NewCustomer<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
    pub price_level_id: Option<i32>,
}

impl From<Customer> for DomainCustomer {
    fn from(value: Customer) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            name: value.name,
            email: value.email,
            price_level_id: value.price_level_id,
        }
    }
}

impl<'a> From<&'a DomainNewCustomer> for NewCustomer<'a> {
    fn from(value: &'a DomainNewCustomer) -> Self {
        Self {
            hub_id: value.hub_id,
            name: value.name.as_str(),
            email: value.email.as_str(),
            price_level_id: value.price_level_id,
        }
    }
}
