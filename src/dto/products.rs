use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::{
    category::Category, price_level::PriceLevel, product::Product,
    product_price_level::ProductPriceLevelRate, tag::Tag, vendor::Vendor,
};
use pushkind_common::pagination::Paginated;

/// Query parameters accepted by the products index page.
#[derive(Debug, Default, Deserialize)]
pub struct ProductsQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page requested by the UI (1-based).
    pub page: Option<usize>,
    /// Whether archived items should be included in the response.
    #[serde(default)]
    pub show_archived: bool,
}

/// Data required to build the products collection resource payload.
pub struct ProductsPageData {
    /// Paginated list of products displayed in the table.
    pub products: Paginated<ProductView>,
    /// Raw list items used to build resource responses.
    pub product_items: Vec<ProductView>,
    /// Total items matching the current query before pagination.
    pub total_items: usize,
    /// Search query echoed back to the view when present.
    pub search: Option<String>,
    /// All price levels used to render the modal form.
    pub price_levels: Vec<PriceLevel>,
    /// All available categories for the add product form.
    pub categories: Vec<Category>,
    /// All available tags for the edit product modal.
    pub tags: Vec<Tag>,
    /// All available vendors for assigning ownership.
    pub vendors: Vec<Vendor>,
    /// Whether archived items were requested.
    pub show_archived: bool,
}

/// Product view model exposed through collection and details APIs.
#[derive(Clone, Debug, Serialize)]
pub struct ProductView {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub units: Option<String>,
    pub currency: String,
    pub is_archived: bool,
    pub category_id: Option<i32>,
    pub vendor_id: Option<i32>,
    pub vendor_name: Option<String>,
    pub category_name: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
    pub price_levels: Vec<ProductPriceLevelView>,
    pub tags: Vec<ProductTagView>,
    pub image_urls: Vec<String>,
    pub amount: Option<f32>,
}

impl ProductView {
    pub fn from_product(
        product: Product,
        level_lookup: &HashMap<i32, &PriceLevel>,
        category_lookup: &HashMap<i32, String>,
        vendor_lookup: &HashMap<i32, String>,
    ) -> Self {
        let Product {
            id,
            hub_id,
            name,
            sku,
            description,
            units,
            currency,
            is_archived,
            category_id,
            vendor_id,
            price_levels,
            tags,
            image_urls,
            created_at: _,
            updated_at,
            amount,
        } = product;

        let price_levels = price_levels
            .into_iter()
            .flat_map(|rate| ProductPriceLevelView::from_rate(rate, level_lookup))
            .collect();

        let tags = tags.into_iter().map(ProductTagView::from_tag).collect();

        Self {
            id: id.get(),
            hub_id: hub_id.get(),
            name: name.as_str().to_string(),
            sku: sku.map(|sku| sku.as_str().to_string()),
            description: description.map(|d| d.into_inner()),
            units: units.map(|units| units.as_str().to_string()),
            currency: currency.as_str().to_string(),
            is_archived,
            category_id: category_id.map(|id| id.get()),
            vendor_id: vendor_id.map(|id| id.get()),
            vendor_name: vendor_id.and_then(|id| vendor_lookup.get(&id.get()).cloned()),
            category_name: category_id.and_then(|id| category_lookup.get(&id.get()).cloned()),
            updated_at,
            price_levels,
            tags,
            image_urls: image_urls.into_iter().map(|url| url.into_inner()).collect(),
            amount: amount.map(|a| a.get()),
        }
    }
}

/// View model for a product price level entry.
#[derive(Clone, Debug, Serialize)]
pub struct ProductPriceLevelView {
    pub price_level_id: i32,
    pub price_level_name: String,
    pub price_cents: i32,
    pub price_formatted: String,
}

impl ProductPriceLevelView {
    fn from_rate(
        rate: ProductPriceLevelRate,
        level_lookup: &HashMap<i32, &PriceLevel>,
    ) -> Option<Self> {
        let level = level_lookup.get(&rate.price_level_id.get())?;
        let price_formatted = format!("{:.2}", rate.price_cents.get() as f64 / 100.0);

        Some(Self {
            price_level_id: rate.price_level_id.get(),
            price_level_name: level.name.as_str().to_string(),
            price_cents: rate.price_cents.get(),
            price_formatted,
        })
    }
}

/// View model for a product tag entry.
#[derive(Clone, Debug, Serialize)]
pub struct ProductTagView {
    pub id: i32,
    pub name: String,
}

impl ProductTagView {
    fn from_tag(tag: Tag) -> Self {
        Self {
            id: tag.id.get(),
            name: tag.name.as_str().to_string(),
        }
    }
}
