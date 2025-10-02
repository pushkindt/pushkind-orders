use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

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
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: i64,
    /// ISO 4217 currency code associated with the product price.
    pub currency: String,
    /// Flag indicating whether the product has been archived.
    pub is_archived: bool,
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
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: i64,
    /// ISO 4217 currency code associated with the product price.
    pub currency: String,
    /// Timestamp captured when the product payload was created.
    pub updated_at: NaiveDateTime,
}

impl NewProduct {
    /// Build a new product payload with the supplied details and current timestamp.
    pub fn new(
        hub_id: i32,
        name: impl Into<String>,
        price_cents: i64,
        currency: impl Into<String>,
    ) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            name: name.into(),
            sku: None,
            description: None,
            price_cents,
            currency: currency.into(),
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
}

/// Patch data applied when updating an existing product.
#[derive(Debug, Clone)]
pub struct UpdateProduct {
    /// Optional name update.
    pub name: Option<String>,
    /// Optional SKU update.
    pub sku: Option<Option<String>>,
    /// Optional description update.
    pub description: Option<Option<String>>,
    /// Optional price update in the smallest currency unit.
    pub price_cents: Option<i64>,
    /// Optional currency update.
    pub currency: Option<String>,
    /// Whether the product should be archived or restored.
    pub is_archived: Option<bool>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl Default for UpdateProduct {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateProduct {
    /// Create a new patch object with no changes applied yet.
    pub fn new() -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            name: None,
            sku: None,
            description: None,
            price_cents: None,
            currency: None,
            is_archived: None,
            updated_at: now,
        }
    }

    /// Update the product name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Update the SKU, using `None` to clear an existing value.
    pub fn sku(mut self, sku: Option<impl Into<String>>) -> Self {
        self.sku = Some(sku.map(|value| value.into()));
        self
    }

    /// Update the product description, using `None` to clear an existing value.
    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = Some(description.map(|value| value.into()));
        self
    }

    /// Update the product price.
    pub fn price_cents(mut self, price_cents: i64) -> Self {
        self.price_cents = Some(price_cents);
        self
    }

    /// Update the currency used for the product.
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Archive or restore the product.
    pub fn archived(mut self, is_archived: bool) -> Self {
        self.is_archived = Some(is_archived);
        self
    }
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
