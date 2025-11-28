//! Tag domain models for product labeling and organization.

use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::types::{HubId, TagId, TagName, TypeConstraintError};

/// Domain representation of a reusable tag that can be attached to multiple products.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    /// Unique identifier of the tag.
    pub id: TagId,
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the tag.
    pub name: TagName,
    /// Timestamp for when the tag record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the tag record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new tag for a hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTag {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the tag.
    pub name: TagName,
}

impl NewTag {
    /// Construct a new tag payload; callers must supply pre-normalised fields.
    pub fn new(hub_id: HubId, name: TagName) -> Self {
        Self { hub_id, name }
    }

    /// Attempt to construct a tag payload by enforcing domain constraints.
    pub fn try_new(hub_id: i32, name: impl Into<String>) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?, TagName::new(name)?))
    }
}

/// Patch data applied when updating an existing tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTag {
    /// Updated human-readable name of the tag.
    pub name: TagName,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl UpdateTag {
    /// Construct a new patch payload; callers must supply pre-normalised fields.
    pub fn new(name: TagName) -> Self {
        let updated_at = chrono::Utc::now().naive_utc();
        Self { name, updated_at }
    }

    /// Attempt to build a tag update by enforcing domain constraints.
    pub fn try_new(name: impl Into<String>) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(TagName::new(name)?))
    }
}

/// Query definition used to list tags for a hub.
#[derive(Debug, Clone)]
pub struct TagListQuery {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional case-insensitive substring search.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl TagListQuery {
    /// Construct a query that targets all tags belonging to `hub_id`.
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

    /// Filter the results by a search term applied to the tag name.
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
