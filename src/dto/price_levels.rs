use serde::{Deserialize, Serialize};

use crate::domain::{category::Category, customer::Customer, price_level::PriceLevel};

/// Query parameters accepted by the price levels index page.
#[derive(Debug, Default, Deserialize)]
pub struct PriceLevelsQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
}

/// Data required to render the price levels index template.
pub struct PriceLevelsPageData {
    /// Paginated list of price levels to show in the table.
    pub price_levels: Vec<PriceLevel>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
    /// Categories available for price level targeting.
    pub categories: Vec<Category>,
}

/// Saved price level assignment for a specific customer.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientPriceLevelAssignment {
    /// Phone number stored for the customer.
    pub phone: String,
    /// Selected price level identifier, if any.
    pub price_level_id: Option<i32>,
}

impl From<Customer> for ClientPriceLevelAssignment {
    fn from(customer: Customer) -> Self {
        Self {
            phone: customer.phone.as_str().to_string(),
            price_level_id: customer.price_level_id.map(|id| id.get()),
        }
    }
}

/// Aggregated client assignments together with the hub default.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientPriceLevelAssignments {
    /// Owning hub identifier for the assignments.
    pub hub_id: i32,
    /// Default price level identifier configured for the hub.
    pub default_price_level_id: Option<i32>,
    /// Saved assignments for customers belonging to the hub.
    pub assignments: Vec<ClientPriceLevelAssignment>,
}
