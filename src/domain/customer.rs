//! Customer domain models and query builders.

use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    CustomerId, CustomerName, HubId, PhoneNumber, PriceLevelId, TypeConstraintError, UserEmail,
};

/// Domain representation of a customer that belongs to a hub.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Customer {
    /// Unique identifier of the customer.
    pub id: CustomerId,
    /// Hub identifier that owns the customer.
    pub hub_id: HubId,
    /// Human-friendly display name of the customer.
    pub name: CustomerName,
    /// Primary email address expected to be supplied in lowercase for comparisons.
    pub email: Option<UserEmail>,
    /// Contact phone number associated with the customer in E.164 format.
    pub phone: PhoneNumber,
    /// Optional price level assigned to the customer; falls back to the hub default when absent.
    pub price_level_id: Option<PriceLevelId>,
}

/// Payload required to insert a new customer for a hub.
#[derive(Debug, Clone, Deserialize)]
pub struct NewCustomer {
    /// Hub identifier that owns the customer.
    pub hub_id: HubId,
    /// Human-friendly display name of the customer.
    pub name: CustomerName,
    /// Primary email address expected to be supplied in lowercase for comparisons.
    pub email: Option<UserEmail>,
    /// Contact phone number associated with the customer.
    pub phone: PhoneNumber,
    /// Optional price level assigned to the customer.
    pub price_level_id: Option<PriceLevelId>,
}

impl NewCustomer {
    /// Build a new customer payload from pre-sanitised inputs supplied by the caller.
    #[must_use]
    pub fn new(hub_id: HubId, name: CustomerName, phone: PhoneNumber) -> Self {
        Self {
            hub_id,
            name,
            email: None,
            phone,
            price_level_id: None,
        }
    }

    /// Attempt to build a new customer payload while enforcing domain constraints.
    pub fn try_new(
        hub_id: i32,
        name: impl Into<String>,
        phone: impl Into<String>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            CustomerName::new(name)?,
            PhoneNumber::new(phone)?,
        ))
    }

    /// Attach an email address to the customer payload.
    pub fn with_email(mut self, email: impl Into<String>) -> Result<Self, TypeConstraintError> {
        self.email = Some(UserEmail::new(email)?);
        Ok(self)
    }

    /// Attach a price level identifier to the customer payload.
    #[must_use]
    pub fn with_price_level_id(mut self, price_level_id: PriceLevelId) -> Self {
        self.price_level_id = Some(price_level_id);
        self
    }

    /// Attach a price level identifier from a raw integer.
    pub fn try_with_price_level_id(
        mut self,
        price_level_id: i32,
    ) -> Result<Self, TypeConstraintError> {
        self.price_level_id = Some(PriceLevelId::new(price_level_id)?);
        Ok(self)
    }
}

/// Query definition used to list customers for a hub.
#[derive(Debug, Clone)]
pub struct CustomerListQuery {
    pub hub_id: HubId,
    pub search: Option<String>,
    pub price_level_id: Option<PriceLevelId>,
    pub pagination: Option<Pagination>,
}

impl CustomerListQuery {
    /// Construct a query that targets all customers belonging to `hub_id`.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            search: None,
            price_level_id: None,
            pagination: None,
        }
    }

    /// Attempt to construct a query from a raw hub identifier.
    pub fn try_new(hub_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?))
    }

    /// Filter the results by a case-insensitive search on name or email fields.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Restrict the results to customers assigned to the specified price level.
    pub fn price_level(mut self, price_level_id: PriceLevelId) -> Self {
        self.price_level_id = Some(price_level_id);
        self
    }

    /// Attempt to apply a price level filter from a raw identifier.
    pub fn try_price_level(mut self, price_level_id: i32) -> Result<Self, TypeConstraintError> {
        self.price_level_id = Some(PriceLevelId::new(price_level_id)?);
        Ok(self)
    }

    /// Apply pagination to the query with the given page number and page size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}
