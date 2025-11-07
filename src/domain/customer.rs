use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

/// Domain representation of a customer that belongs to a hub.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Customer {
    /// Unique identifier of the customer.
    pub id: i32,
    /// Hub identifier that owns the customer.
    pub hub_id: i32,
    /// Human-friendly display name of the customer.
    pub name: String,
    /// Primary email address expected to be supplied in lowercase for comparisons.
    pub email: Option<String>,
    /// Contact phone number associated with the customer in E.164 format.
    pub phone: String,
    /// Optional price level assigned to the customer; falls back to the hub default when absent.
    pub price_level_id: Option<i32>,
}

/// Payload required to insert a new customer for a hub.
#[derive(Debug, Clone, Deserialize)]
pub struct NewCustomer {
    /// Hub identifier that owns the customer.
    pub hub_id: i32,
    /// Human-friendly display name of the customer.
    pub name: String,
    /// Primary email address expected to be supplied in lowercase for comparisons.
    pub email: Option<String>,
    /// Contact phone number associated with the customer.
    pub phone: String,
    /// Optional price level assigned to the customer.
    pub price_level_id: Option<i32>,
}

impl NewCustomer {
    /// Build a new customer payload from pre-sanitised inputs supplied by the caller.
    #[must_use]
    pub fn new(hub_id: i32, name: impl Into<String>, phone: impl Into<String>) -> Self {
        let name = name.into();
        let phone = phone.into();
        Self {
            hub_id,
            name,
            email: None,
            phone,
            price_level_id: None,
        }
    }

    /// Attach an email address to the customer payload.
    #[must_use]
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Attach a price level identifier to the customer payload.
    #[must_use]
    pub fn with_price_level_id(mut self, price_level_id: i32) -> Self {
        self.price_level_id = Some(price_level_id);
        self
    }
}

#[derive(Debug, Clone)]
/// Query definition used to list customers for a hub.
pub struct CustomerListQuery {
    pub hub_id: i32,
    pub search: Option<String>,
    pub price_level_id: Option<i32>,
    pub pagination: Option<Pagination>,
}

impl CustomerListQuery {
    /// Construct a query that targets all customers belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            search: None,
            price_level_id: None,
            pagination: None,
        }
    }

    /// Filter the results by a case-insensitive search on name or email fields.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Restrict the results to customers assigned to the specified price level.
    pub fn price_level(mut self, price_level_id: i32) -> Self {
        self.price_level_id = Some(price_level_id);
        self
    }

    /// Apply pagination to the query with the given page number and page size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}
