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
    /// Primary email address stored in lowercase for comparisons.
    pub email: String,
    /// Optional price level assigned to the customer.
    /// If None, than a default price level is used
    pub price_level_id: Option<i32>,
}

/// Payload required to insert a new customer for a hub.
#[derive(Debug, Clone, Deserialize)]
pub struct NewCustomer {
    /// Hub identifier that owns the customer.
    pub hub_id: i32,
    /// Human-friendly display name of the customer.
    pub name: String,
    /// Primary email address stored in lowercase for comparisons.
    pub email: String,
    /// Optional price level assigned to the customer.
    pub price_level_id: Option<i32>,
}

impl NewCustomer {
    /// Build a new customer payload while normalising the email to lowercase.
    #[must_use]
    pub fn new(hub_id: i32, name: impl Into<String>, email: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        let email = email.into().trim().to_lowercase();
        Self {
            hub_id,
            name,
            email,
            price_level_id: None,
        }
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
