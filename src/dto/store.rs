use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;

use crate::domain::{
    category::Category,
    customer::Customer,
    order::{Order, OrderProduct, OrderStatus},
    product::{Product, ProductListQuery},
    product_price_level::ProductPriceLevelRate,
    tag::Tag,
    types::{CategoryId, HubId},
};

/// Minimal representation of a category exposed to the storefront.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreCategory {
    /// Identifier of the category.
    pub id: i32,
    /// Parent identifier when the category belongs to a tree.
    pub parent_id: Option<i32>,
    /// Name displayed to users.
    pub name: String,
    /// Optional descriptive text.
    pub description: Option<String>,
    // Optional image_url serialized as imageUrl
    pub image_url: Option<String>,
}

impl From<Category> for StoreCategory {
    fn from(value: Category) -> Self {
        Self {
            id: value.id.get(),
            parent_id: value.parent_id.map(|id| id.get()),
            name: value.name.as_str().to_string(),
            description: value.description.map(|desc| desc.as_str().to_string()),
            image_url: value.image_url.map(|url| url.as_str().to_string()),
        }
    }
}

/// Minimal representation of a tag exposed to the storefront.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreTag {
    /// Identifier of the tag.
    pub id: i32,
    /// Name displayed to users.
    pub name: String,
}

impl From<Tag> for StoreTag {
    fn from(value: Tag) -> Self {
        Self {
            id: value.id.get(),
            name: value.name.as_str().to_string(),
        }
    }
}

/// Response returned after an OTP request has been accepted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOtpAcceptResponse {
    /// Indicates whether the request has been accepted.
    pub success: bool,
}

/// Response returned after an OTP request has been accepted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOtpVerifyResponse {
    /// Indicates whether the request has been verified.
    pub success: bool,
    pub customer: Customer,
}

/// Optional filters that can be applied when listing store categories.
#[derive(Debug, Clone, Default)]
pub struct StoreCategoryFilters {
    /// Only include categories belonging to this parent identifier.
    pub parent_id: Option<i32>,
}

/// Product payload formatted for storefront consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreProduct {
    /// Identifier of the product.
    pub id: i32,
    /// Optional category identifier used for grouping.
    pub category_id: Option<i32>,
    /// Name displayed to users.
    pub name: String,
    /// Optional stock keeping unit identifier.
    pub sku: Option<String>,
    /// Optional longer description.
    pub description: Option<String>,
    /// Optional unit of measure.
    pub units: Option<String>,
    /// ISO 4217 currency code associated with the product.
    pub currency: String,
    /// Price in smallest currency unit for the hub default price level, when configured.
    pub price_cents: Option<i32>,
    /// Tags attached to the product.
    pub tags: Vec<StoreTag>,
    /// Image URLs attached to the product.
    pub image_urls: Vec<String>,
    /// Timestamp representing when the product was last updated.
    pub updated_at: NaiveDateTime,
    /// Optional amount per unit
    pub amount: Option<f32>,
}

impl StoreProduct {
    pub fn resolve_price_cents(
        price_levels: &[ProductPriceLevelRate],
        customer_price_level_id: Option<i32>,
        default_price_level_id: Option<i32>,
    ) -> Option<i32> {
        if let Some(level_id) = customer_price_level_id
            && let Some(rate) = price_levels
                .iter()
                .find(|rate| rate.price_level_id.get() == level_id)
        {
            return Some(rate.price_cents.get());
        }

        if let Some(level_id) = default_price_level_id
            && let Some(rate) = price_levels
                .iter()
                .find(|rate| rate.price_level_id.get() == level_id)
        {
            return Some(rate.price_cents.get());
        }

        None
    }

    pub fn from_domain(
        value: Product,
        customer_price_level_id: Option<i32>,
        default_price_level_id: Option<i32>,
    ) -> Self {
        let Product {
            id,
            hub_id: _,
            name,
            sku,
            description,
            units,
            currency,
            is_archived: _,
            category_id,
            price_levels,
            tags,
            image_urls,
            created_at: _,
            updated_at,
            amount,
        } = value;

        let price_cents = Self::resolve_price_cents(
            &price_levels,
            customer_price_level_id,
            default_price_level_id,
        );

        Self {
            id: id.get(),
            category_id: category_id.map(|id| id.get()),
            name: name.as_str().to_string(),
            sku: sku.map(|sku| sku.as_str().to_string()),
            description: description.map(|d| d.into_inner()),
            units: units.map(|units| units.as_str().to_string()),
            currency: currency.as_str().to_string(),
            price_cents,
            tags: tags.into_iter().map(StoreTag::from).collect(),
            image_urls: image_urls.into_iter().map(|url| url.into_inner()).collect(),
            updated_at,
            amount: amount.map(|a| a.get()),
        }
    }
}

impl From<Product> for StoreProduct {
    fn from(value: Product) -> Self {
        Self::from_domain(value, None, None)
    }
}

/// Optional filters that can be applied when listing store products.
#[derive(Debug, Clone, Default)]
pub struct StoreProductFilters {
    /// Only include products belonging to this category.
    pub category_id: Option<i32>,
    /// Filter products by a search term applied to the name and description.
    pub search: Option<String>,
    /// Fetch a specific page of products.
    pub page: Option<usize>,
}

impl StoreProductFilters {
    pub fn into_query(self, hub_id: HubId) -> ProductListQuery {
        let mut query = ProductListQuery::new(hub_id);

        query = match self.category_id.and_then(|id| CategoryId::new(id).ok()) {
            Some(category_id) => query.with_category_id(category_id),
            None => query.only_without_category(),
        };

        if let Some(search) = self
            .search
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            query = query.search(search);
        }

        if let Some(page) = self.page.filter(|page| *page > 0) {
            query = query.paginate(page, DEFAULT_ITEMS_PER_PAGE);
        }

        query
    }
}

/// Order payload formatted for storefront consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOrder {
    /// Unique identifier of the order.
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional reference to the customer placing the order.
    pub customer_id: Option<i32>,
    /// External human-friendly reference for the order.
    pub reference: Option<String>,
    /// Current lifecycle status of the order.
    pub status: OrderStatus,
    /// Optional notes supplied by the operator.
    pub notes: Option<String>,
    /// Total amount represented in the smallest currency unit (for example cents).
    pub total_cents: i32,
    /// ISO 4217 currency code used for the order total.
    pub currency: String,
    /// Product snapshots captured when the order was created.
    pub products: Vec<StoreOrderProduct>,
    /// Timestamp for when the order record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the order record.
    pub updated_at: NaiveDateTime,
}

impl From<Order> for StoreOrder {
    fn from(value: Order) -> Self {
        Self {
            id: value.id.get(),
            hub_id: value.hub_id.get(),
            customer_id: value.customer_id.map(|id| id.get()),
            reference: value.reference.map(|r| r.into_inner()),
            status: value.status,
            notes: value.notes.map(|n| n.into_inner()),
            total_cents: value.total_cents.get(),
            currency: value.currency.into_inner(),
            products: value
                .products
                .into_iter()
                .map(StoreOrderProduct::from)
                .collect(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

/// Ordered product payload formatted for storefront consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOrderProduct {
    /// Identifier of the original product, if it still exists.
    pub product_id: Option<i32>,
    /// Human-readable name captured at the time of ordering.
    pub name: String,
    /// Stock keeping unit captured at the time of ordering.
    pub sku: Option<String>,
    /// Description captured at the time of ordering.
    pub description: Option<String>,
    /// Price represented in the smallest currency unit for the ordered quantity.
    pub price_cents: i32,
    /// ISO 4217 currency captured at the time of ordering.
    pub currency: String,
    /// Quantity of the product ordered.
    pub quantity: i32,
}

impl From<OrderProduct> for StoreOrderProduct {
    fn from(value: OrderProduct) -> Self {
        Self {
            product_id: value.product_id.map(|id| id.get()),
            name: value.name.into_inner(),
            sku: value.sku.map(|s| s.into_inner()),
            description: value.description.map(|d| d.into_inner()),
            price_cents: value.price_cents.get(),
            currency: value.currency.into_inner(),
            quantity: value.quantity.get(),
        }
    }
}
