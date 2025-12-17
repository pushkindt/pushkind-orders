use serde::{Deserialize, Serialize};

use crate::domain::{customer::Customer, order::Order};

/// Adjustment payload for updating order product approvals.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderProductApprovalPayload {
    /// Associated product identifier captured at order time.
    pub product_id: i32,
    /// Approved quantity for fulfillment.
    pub approved_quantity: i32,
}

/// Aggregated order details with optional customer information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderDetails {
    /// Order record with product snapshots.
    pub order: Order,
    /// Customer assigned to the order, if the reference still exists.
    pub customer: Option<Customer>,
}
