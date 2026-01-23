//! Product domain models with price levels, tags, and category associations.

use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::{
    product_price_level::ProductPriceLevelRate,
    tag::Tag,
    types::{
        CategoryId, CurrencyCode, HubId, ImageUrl, PriceLevelId, ProductAmount, ProductDescription,
        ProductId, ProductName, ProductSku, ProductUnits, TagId, TypeConstraintError, VendorId,
    },
};

/// Domain representation of a product that can be managed by a hub.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    /// Unique identifier of the product.
    pub id: ProductId,
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the product.
    pub name: ProductName,
    /// Optional stock keeping unit identifier.
    pub sku: Option<ProductSku>,
    /// Optional longer description shown to users.
    pub description: Option<ProductDescription>,
    /// Optional unit of measure for the product (e.g. `kg`, `pack`).
    pub units: Option<ProductUnits>,
    /// ISO 4217 currency code used when assigning prices to this product.
    pub currency: CurrencyCode,
    /// Flag indicating whether the product has been archived.
    pub is_archived: bool,
    /// Optional identifier of the category the product belongs to.
    pub category_id: Option<CategoryId>,
    /// Price level rates configured for the product.
    pub price_levels: Vec<ProductPriceLevelRate>,
    /// Tags associated with the product.
    pub tags: Vec<Tag>,
    /// Image URLS for the product
    pub image_urls: Vec<ImageUrl>,
    /// Optional amount per unit
    pub amount: Option<ProductAmount>,
    /// Timestamp for when the product record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the product record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new product for a hub.
#[derive(Debug, Clone)]
pub struct NewProduct {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable name of the product.
    pub name: ProductName,
    /// Optional stock keeping unit identifier.
    pub sku: Option<ProductSku>,
    /// Optional longer description shown to users.
    pub description: Option<ProductDescription>,
    /// Optional unit of measure for the product (e.g. `kg`, `pack`).
    pub units: Option<ProductUnits>,
    /// Optional amount per unit
    pub amount: Option<ProductAmount>,
    /// ISO 4217 currency code used when assigning prices to this product.
    pub currency: CurrencyCode,
    /// Optional identifier of the category the product belongs to.
    pub category_id: Option<CategoryId>,
}

impl NewProduct {
    /// Build a new product payload; callers must supply pre-sanitised strings and optional fields default to `None`.
    pub fn new(hub_id: HubId, name: ProductName, currency: CurrencyCode) -> Self {
        Self {
            hub_id,
            name,
            sku: None,
            description: None,
            units: None,
            currency,
            category_id: None,
            amount: None,
        }
    }

    /// Attempt to build a new product from raw inputs.
    pub fn try_new(
        hub_id: i32,
        name: impl Into<String>,
        currency: impl Into<String>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            ProductName::new(name)?,
            CurrencyCode::new(currency)?,
        ))
    }

    /// Attach an SKU identifier to the product payload.
    pub fn with_sku(mut self, sku: ProductSku) -> Self {
        self.sku = Some(sku);
        self
    }

    /// Attach a descriptive text to the product payload.
    pub fn with_description(mut self, description: ProductDescription) -> Self {
        self.description = Some(description);
        self
    }

    /// Attach a unit of measure to the product payload.
    pub fn with_units(mut self, units: ProductUnits) -> Self {
        self.units = Some(units);
        self
    }

    /// Assign the product to a category.
    pub fn with_category_id(mut self, category_id: CategoryId) -> Self {
        self.category_id = Some(category_id);
        self
    }

    /// Attach amount to the product payload.
    pub fn with_amount(mut self, amount: ProductAmount) -> Self {
        self.amount = Some(amount);
        self
    }
}

/// Patch data applied when updating an existing product.
#[derive(Debug, Clone)]
pub struct UpdateProduct {
    /// Name update.
    pub name: ProductName,
    /// Optional SKU update.
    pub sku: Option<ProductSku>,
    /// Optional description update.
    pub description: Option<ProductDescription>,
    /// Optional unit of measure update.
    pub units: Option<ProductUnits>,
    /// Optional amount per unit
    pub amount: Option<ProductAmount>,
    /// Currency update.
    pub currency: CurrencyCode,
    /// Whether the product should be archived or restored.
    pub is_archived: bool,
    /// Optional identifier of the category the product belongs to.
    pub category_id: Option<CategoryId>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl UpdateProduct {
    /// Build a patch payload with the supplied details and current timestamp.
    pub fn new(name: ProductName, currency: CurrencyCode, is_archived: bool) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            name,
            sku: None,
            description: None,
            units: None,
            currency,
            is_archived,
            category_id: None,
            updated_at: now,
            amount: None,
        }
    }

    /// Attempt to construct an update payload from raw values.
    pub fn try_new(
        name: impl Into<String>,
        currency: impl Into<String>,
        is_archived: bool,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ProductName::new(name)?,
            CurrencyCode::new(currency)?,
            is_archived,
        ))
    }

    /// Attach an SKU identifier to the patch payload.
    pub fn with_sku(mut self, sku: ProductSku) -> Self {
        self.sku = Some(sku);
        self
    }

    /// Attach a descriptive text to the patch payload.
    pub fn with_description(mut self, description: ProductDescription) -> Self {
        self.description = Some(description);
        self
    }

    /// Attach a unit of measure to the patch payload.
    pub fn with_units(mut self, units: ProductUnits) -> Self {
        self.units = Some(units);
        self
    }

    /// Assign the product to a category.
    pub fn with_category_id(mut self, category_id: CategoryId) -> Self {
        self.category_id = Some(category_id);
        self
    }

    /// Attach amount to the product payload.
    pub fn with_amount(mut self, amount: ProductAmount) -> Self {
        self.amount = Some(amount);
        self
    }
}

/// Query definition used to list products for a hub.
#[derive(Debug, Clone)]
pub struct ProductListQuery {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional identifier of the category the products belong to.
    pub category_id: Option<CategoryId>,
    /// Optional tag identifier to filter products by.
    pub tag_id: Option<TagId>,
    /// Whether only products without an assigned category should be included.
    pub only_without_category: bool,
    /// Optional name or description search term.
    pub search: Option<String>,
    /// Optional lower bound for product amount.
    pub min_amount: Option<ProductAmount>,
    /// Optional upper bound for product amount.
    pub max_amount: Option<ProductAmount>,
    /// Optional exact SKU filter.
    pub sku: Option<ProductSku>,
    /// Optional price level identifier used to require a matching price level assignment.
    pub price_level_id: Option<PriceLevelId>,
    /// Optional vendor identifier used to filter products by vendor.
    pub vendor_id: Option<VendorId>,
    /// Whether archived products should be included in the results.
    pub include_archived: bool,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl ProductListQuery {
    /// Construct a query that targets all products belonging to `hub_id`.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            category_id: None,
            only_without_category: false,
            search: None,
            min_amount: None,
            max_amount: None,
            sku: None,
            price_level_id: None,
            include_archived: false,
            pagination: None,
            tag_id: None,
            vendor_id: None,
        }
    }

    /// Attempt to build from raw hub identifier.
    pub fn try_new(hub_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?))
    }

    /// Filter the results by a search term applied to the name or description.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.only_without_category = false;
        self.search = Some(term.into());
        self
    }

    /// Filter results by a minimum amount (inclusive).
    pub fn with_min_amount(mut self, amount: ProductAmount) -> Self {
        self.min_amount = Some(amount);
        self
    }

    /// Filter results by a maximum amount (inclusive).
    pub fn with_max_amount(mut self, amount: ProductAmount) -> Self {
        self.max_amount = Some(amount);
        self
    }

    /// Filter the results by category.
    pub fn with_category_id(mut self, category_id: CategoryId) -> Self {
        self.category_id = Some(category_id);
        self.only_without_category = false;
        self
    }

    /// Filter the results by tag.
    pub fn with_tag_id(mut self, tag_id: TagId) -> Self {
        self.tag_id = Some(tag_id);
        self.only_without_category = false;
        self
    }

    /// Restrict the results to products that do not have a category.
    pub fn only_without_category(mut self) -> Self {
        self.category_id = None;
        self.only_without_category = true;
        self
    }

    /// Filter the results by an exact SKU match.
    pub fn sku(mut self, sku: ProductSku) -> Self {
        self.sku = Some(sku);
        self.only_without_category = false;
        self
    }

    /// Restrict results to products that have a price level assignment for the specified level.
    pub fn with_price_level_id(mut self, price_level_id: PriceLevelId) -> Self {
        self.price_level_id = Some(price_level_id);
        self
    }

    /// Filter the results by vendor identifier.
    pub fn with_vendor_id(mut self, vendor_id: VendorId) -> Self {
        self.vendor_id = Some(vendor_id);
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
