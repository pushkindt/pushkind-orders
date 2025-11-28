use pushkind_common::pagination::Paginated;
use serde::Deserialize;

use crate::domain::order::Order;

/// Query parameters accepted by the index page service.
#[derive(Debug, Default, Deserialize)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
}

/// Data required to render the main index template.
pub struct IndexPageData {
    /// Paginated list of orders to show in the table.
    pub orders: Paginated<Order>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}
