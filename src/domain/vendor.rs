//! Vendor domain models for product labeling and organization.

use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::types::{HubId, TypeConstraintError, VendorId, VendorName};

/// Domain representation of a reusable vendor that can be attached to multiple products.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vendor {
    /// Unique identifier of the vendor.
    pub id: VendorId,
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the vendor.
    pub name: VendorName,
    /// Timestamp for when the vendor record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the vendor record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new vendor for a hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVendor {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the vendor.
    pub name: VendorName,
}

impl NewVendor {
    /// Construct a new vendor payload; callers must supply pre-normalised fields.
    pub fn new(hub_id: HubId, name: VendorName) -> Self {
        Self { hub_id, name }
    }

    /// Attempt to construct a vendor payload by enforcing domain constraints.
    pub fn try_new(hub_id: i32, name: impl Into<String>) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?, VendorName::new(name)?))
    }
}

/// Patch data applied when updating an existing vendor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVendor {
    /// Updated human-readable name of the vendor.
    pub name: VendorName,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl UpdateVendor {
    /// Construct a new patch payload; callers must supply pre-normalised fields.
    pub fn new(name: VendorName) -> Self {
        let updated_at = chrono::Utc::now().naive_utc();
        Self { name, updated_at }
    }

    /// Attempt to build a vendor update by enforcing domain constraints.
    pub fn try_new(name: impl Into<String>) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(VendorName::new(name)?))
    }
}

/// Query definition used to list vendors for a hub.
#[derive(Debug, Clone)]
pub struct VendorListQuery {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional case-insensitive substring search.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl VendorListQuery {
    /// Construct a query that targets all vendors belonging to `hub_id`.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }

    /// Attempt to construct a query from a raw hub identifier.
    pub fn try_new(hub_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?))
    }

    /// Filter the results by a search term applied to the vendor name.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Apply pagination to the query with the given page number and page size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}
