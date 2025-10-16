use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

/// Domain representation of a configurable price level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceLevel {
    /// Unique identifier of the price level.
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable name of the price level.
    pub name: String,
    /// Timestamp for when the price level record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the price level record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new price level for a hub.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPriceLevel {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable name of the price level.
    pub name: String,
}

impl NewPriceLevel {
    /// Construct a new price level payload with a trimmed name.
    pub fn new(hub_id: i32, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        Self { hub_id, name }
    }
}

/// Patch data applied when updating an existing price level.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdatePriceLevel {
    /// Optional name update for the price level.
    pub name: String,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

/// Query definition used to list price levels for a hub.
#[derive(Debug, Clone)]
pub struct PriceLevelListQuery {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional case-insensitive substring search.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl PriceLevelListQuery {
    /// Construct a query that targets all price levels belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }

    /// Filter the results by a search term applied to the name.
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
