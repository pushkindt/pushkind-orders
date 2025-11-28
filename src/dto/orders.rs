use serde::{Deserialize, Serialize};

use crate::domain::{customer::Customer, order::Order};

/// Aggregated order details with optional customer information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderDetails {
    /// Order record with product snapshots.
    pub order: Order,
    /// Customer assigned to the order, if the reference still exists.
    pub customer: Option<Customer>,
}
