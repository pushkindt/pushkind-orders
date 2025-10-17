use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::product_price_level::ProductPriceLevelRate;

/// Domain representation of a product that can be managed by a hub.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    /// Unique identifier of the product.
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable name of the product.
    pub name: String,
    /// Optional stock keeping unit identifier.
    pub sku: Option<String>,
    /// Optional longer description shown to users.
    pub description: Option<String>,
    /// Optional unit of measure for the product (e.g. `kg`, `pack`).
    pub units: Option<String>,
    /// ISO 4217 currency code used when assigning prices to this product.
    pub currency: String,
    /// Flag indicating whether the product has been archived.
    pub is_archived: bool,
    /// Optional identifier of the category the product belongs to.
    pub category_id: Option<i32>,
    /// Price level rates configured for the product.
    pub price_levels: Vec<ProductPriceLevelRate>,
    /// Timestamp for when the product record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the product record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new product for a hub.
#[derive(Debug, Clone)]
pub struct NewProduct {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable name of the product.
    pub name: String,
    /// Optional stock keeping unit identifier.
    pub sku: Option<String>,
    /// Optional longer description shown to users.
    pub description: Option<String>,
    /// Optional unit of measure for the product (e.g. `kg`, `pack`).
    pub units: Option<String>,
    /// ISO 4217 currency code used when assigning prices to this product.
    pub currency: String,
    /// Optional identifier of the category the product belongs to.
    pub category_id: Option<i32>,
    /// Timestamp captured when the product payload was created.
    pub updated_at: NaiveDateTime,
}

impl NewProduct {
    /// Build a new product payload with the supplied details and current timestamp.
    pub fn new(hub_id: i32, name: impl Into<String>, currency: impl Into<String>) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            name: name.into(),
            sku: None,
            description: None,
            units: None,
            currency: currency.into(),
            category_id: None,
            updated_at: now,
        }
    }

    /// Attach an SKU identifier to the product payload.
    pub fn with_sku(mut self, sku: impl Into<String>) -> Self {
        self.sku = Some(sku.into());
        self
    }

    /// Attach a descriptive text to the product payload.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Attach a unit of measure to the product payload.
    pub fn with_units(mut self, units: impl Into<String>) -> Self {
        self.units = Some(units.into());
        self
    }

    /// Assign the product to a category.
    pub fn with_category_id(mut self, category_id: i32) -> Self {
        self.category_id = Some(category_id);
        self
    }
}

/// Patch data applied when updating an existing product.
#[derive(Debug, Clone, Default)]
pub struct UpdateProduct {
    /// Optional name update.
    pub name: String,
    /// Optional SKU update.
    pub sku: Option<String>,
    /// Optional description update.
    pub description: Option<String>,
    /// Optional unit of measure update.
    pub units: Option<String>,
    /// Optional currency update.
    pub currency: String,
    /// Whether the product should be archived or restored.
    pub is_archived: bool,
    /// Optional identifier of the category the product belongs to.
    pub category_id: Option<i32>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

/// Query definition used to list products for a hub.
#[derive(Debug, Clone)]
pub struct ProductListQuery {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional name or description search term.
    pub search: Option<String>,
    /// Optional exact SKU filter.
    pub sku: Option<String>,
    /// Whether archived products should be included in the results.
    pub include_archived: bool,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl ProductListQuery {
    /// Construct a query that targets all products belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            search: None,
            sku: None,
            include_archived: false,
            pagination: None,
        }
    }

    /// Filter the results by a search term applied to the name or description.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Filter the results by an exact SKU match.
    pub fn sku(mut self, sku: impl Into<String>) -> Self {
        self.sku = Some(sku.into());
        self
    }

    /// Include archived products in the results.
    pub fn include_archived(mut self) -> Self {
        self.include_archived = true;
        self
    }

    /// Apply pagination to the query with the given page number and page size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}
