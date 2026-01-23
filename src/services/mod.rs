//! Service layer orchestrating domain logic and repository operations.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::ensure_role;

use crate::domain::types::{HubId, UserEmail, VendorId};
use crate::repository::{UserReader, VendorUserReader};
use crate::{ADMIN_ACCESS_ROLE, SERVICE_ACCESS_ROLE, VENDOR_ACCESS_ROLE};

pub mod categories;
pub mod main;
pub mod orders;
pub mod price_levels;
pub mod products;
pub mod store;
pub mod tags;
pub mod vendors;

pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

/// Access scope resolved for hub users.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubAccessScope {
    /// Basic access without special roles.
    Basic,
    /// Full hub access for operators.
    Admin,
    /// Vendor-scoped access for vendor users.
    Vendor { vendor_id: VendorId },
}

fn has_role(user: &AuthenticatedUser, role: &str) -> bool {
    user.roles.iter().any(|item| item == role)
}

/// Enforce full hub operator access.
pub fn ensure_admin(user: &AuthenticatedUser) -> ServiceResult<()> {
    ensure_role(user, ADMIN_ACCESS_ROLE)
}

/// Enforce read-only access to hub configuration pages.
pub fn ensure_catalog_read_access(user: &AuthenticatedUser) -> ServiceResult<()> {
    if has_role(user, SERVICE_ACCESS_ROLE)
        || has_role(user, ADMIN_ACCESS_ROLE)
        || has_role(user, VENDOR_ACCESS_ROLE)
    {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}

/// Resolve the hub access scope for an authenticated user.
pub fn resolve_hub_access<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<HubAccessScope>
where
    R: UserReader + VendorUserReader + ?Sized,
{
    if has_role(user, ADMIN_ACCESS_ROLE) {
        Ok(HubAccessScope::Admin)
    } else if has_role(user, VENDOR_ACCESS_ROLE) {
        let hub_id = HubId::new(user.hub_id)?;
        let email = UserEmail::new(user.email.clone())?;
        let user_record = repo
            .get_user_by_email(&email, hub_id)?
            .ok_or(ServiceError::Unauthorized)?;

        let vendor_id = repo
            .get_vendor_for_user(user_record.id, hub_id)?
            .ok_or(ServiceError::Unauthorized)?;

        Ok(HubAccessScope::Vendor { vendor_id })
    } else if has_role(user, SERVICE_ACCESS_ROLE) {
        Ok(HubAccessScope::Basic)
    } else {
        Err(ServiceError::Unauthorized)
    }
}
