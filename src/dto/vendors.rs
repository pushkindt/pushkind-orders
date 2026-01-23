use serde::{Deserialize, Serialize};

use pushkind_common::pagination::Paginated;

use crate::domain::{user::User, vendor::Vendor};

/// Query parameters accepted by the vendors index page.
#[derive(Debug, Default, Deserialize)]
pub struct VendorQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page requested by the UI (1-based).
    pub page: Option<usize>,
}

/// Data required to render the vendors index template.
pub struct VendorsPageData {
    /// Paginated list of vendors displayed in the table.
    pub vendors: Paginated<Vendor>,
    /// Full list of vendors for assignment dropdowns.
    pub vendor_choices: Vec<Vendor>,
    /// User list with current vendor assignments.
    pub users: Vec<VendorUserView>,
    /// Search query echoed back to the view when present.
    pub search: Option<String>,
}

/// View model exposing a user's vendor assignment.
#[derive(Debug, Clone, Serialize)]
pub struct VendorUserView {
    pub user_id: i32,
    pub name: String,
    pub email: String,
    pub vendor_id: Option<i32>,
    pub vendor_name: Option<String>,
}

impl VendorUserView {
    pub fn from_user(user: User, vendor_id: Option<i32>, vendor_name: Option<String>) -> Self {
        Self {
            user_id: user.id.get(),
            name: user.name.as_str().to_string(),
            email: user.email.as_str().to_string(),
            vendor_id,
            vendor_name,
        }
    }
}
