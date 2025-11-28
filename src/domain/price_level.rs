use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::types::{HubId, PriceLevelId, PriceLevelName, TypeConstraintError};

/// Domain representation of a configurable price level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceLevel {
    /// Unique identifier of the price level.
    pub id: PriceLevelId,
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the price level.
    pub name: PriceLevelName,
    /// Timestamp for when the price level record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the price level record.
    pub updated_at: NaiveDateTime,
    /// A flag indicating if the price level is default.
    /// Only one price level can be default at a time for a hub.
    pub is_default: bool,
}

/// Payload required to insert a new price level for a hub.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPriceLevel {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the price level.
    pub name: PriceLevelName,
    /// Default flag for the price level.
    pub is_default: bool,
}

impl NewPriceLevel {
    /// Construct a new price level payload with a name and is_default.
    pub fn new(hub_id: HubId, name: PriceLevelName, is_default: bool) -> Self {
        Self {
            hub_id,
            name,
            is_default,
        }
    }

    /// Attempt to build a new price level from raw values.
    pub fn try_new(
        hub_id: i32,
        name: impl Into<String>,
        is_default: bool,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            PriceLevelName::new(name)?,
            is_default,
        ))
    }
}

/// Patch data applied when updating an existing price level.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdatePriceLevel {
    /// Name update for the price level.
    pub name: PriceLevelName,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
    /// Default flag update for the price level.
    pub is_default: bool,
}

impl UpdatePriceLevel {
    /// Construct a patch payload with a name and is_default.
    pub fn new(name: PriceLevelName, is_default: bool) -> Self {
        let updated_at = chrono::Utc::now().naive_utc();
        Self {
            name,
            updated_at,
            is_default,
        }
    }

    /// Attempt to construct an update payload from raw name.
    pub fn try_new(name: impl Into<String>, is_default: bool) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(PriceLevelName::new(name)?, is_default))
    }
}

/// Query definition used to list price levels for a hub.
#[derive(Debug, Clone)]
pub struct PriceLevelListQuery {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional case-insensitive substring search.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl PriceLevelListQuery {
    /// Construct a query that targets all price levels belonging to `hub_id`.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }

    /// Attempt to construct from a raw hub id.
    pub fn try_new(hub_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?))
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
