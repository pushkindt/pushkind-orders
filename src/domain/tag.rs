use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

/// Domain representation of a reusable tag that can be attached to multiple products.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    /// Unique identifier of the tag.
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable name of the tag.
    pub name: String,
    /// Timestamp for when the tag record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the tag record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new tag for a hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTag {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable name of the tag.
    pub name: String,
}

impl NewTag {
    /// Construct a new tag payload with a trimmed name.
    pub fn new(hub_id: i32, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        Self { hub_id, name }
    }
}

/// Patch data applied when updating an existing tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTag {
    /// Updated human-readable name of the tag.
    pub name: String,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

/// Query definition used to list tags for a hub.
#[derive(Debug, Clone)]
pub struct TagListQuery {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional case-insensitive substring search.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl TagListQuery {
    /// Construct a query that targets all tags belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
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
