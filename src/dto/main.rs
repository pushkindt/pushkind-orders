use pushkind_common::pagination::Paginated;
use serde::Deserialize;

use crate::domain::order::Order;

/// Query parameters accepted by the index page service.
#[derive(Debug, Default, Deserialize)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Optional order status filter.
    pub status: Option<String>,
    /// Optional lower bound for the updated_at date in YYYY-MM-DD format.
    pub updated_after: Option<String>,
    /// Optional upper bound for the updated_at date in YYYY-MM-DD format.
    pub updated_before: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
}

/// Data required to build the orders index resource payload.
pub struct IndexPageData {
    /// Paginated list of orders to show in the table.
    pub orders: Paginated<Order>,
    /// Search query echoed back to the client when present.
    pub search: Option<String>,
}
