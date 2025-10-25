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
        Self {
            hub_id,
            name: name.into(),
            email: email.into().to_lowercase(),
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
