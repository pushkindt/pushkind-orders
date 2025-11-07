use std::{collections::HashMap, io::Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::{StringRecord, Trim};
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::{
    domain::{
        price_level::PriceLevel,
        product::{NewProduct, UpdateProduct},
    },
    forms::{empty_id_as_none, sanitize_text},
};

/// Maximum allowed length for a product name.
const NAME_MAX_LEN: usize = 128;
const NAME_MAX_LEN_VALIDATOR: u64 = NAME_MAX_LEN as u64;

/// Maximum allowed length for a SKU.
const SKU_MAX_LEN: usize = 64;
const SKU_MAX_LEN_VALIDATOR: u64 = SKU_MAX_LEN as u64;

/// Maximum allowed length for a unit of measure descriptor.
const UNITS_MAX_LEN: usize = 32;
const UNITS_MAX_LEN_VALIDATOR: u64 = UNITS_MAX_LEN as u64;

/// ISO 4217 currency codes are three ASCII alphabetic characters.
const CURRENCY_CODE_LEN: usize = 3;
const CURRENCY_CODE_LEN_VALIDATOR: u64 = CURRENCY_CODE_LEN as u64;

fn sanitize_image_urls(input: Option<String>) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(raw) = input {
        for line in raw.lines() {
            if let Some(url) = sanitize_text(line) {
                urls.push(url);
            }
        }
    }
    urls
}

/// Result type returned by the product form helpers.
pub type ProductFormResult<T> = Result<T, ProductFormError>;

/// Errors that can occur while processing product forms.
#[derive(Debug, Error)]
pub enum ProductFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The provided name is empty after sanitization.
    #[error("product name cannot be empty")]
    EmptyName,
    /// The provided currency code is invalid.
    #[error("invalid currency code")]
    InvalidCurrency,
    /// The uploaded CSV is missing required columns.
    #[error("upload is missing the required `name` or `currency` headers")]
    MissingRequiredHeaders,
    /// A CSV row did not include a product name.
    #[error("row {row} is missing a product name")]
    UploadMissingName { row: usize },
    /// A CSV row did not include a currency code.
    #[error("row {row} is missing a currency code")]
    UploadMissingCurrency { row: usize },
    /// A CSV row contained an invalid currency code.
    #[error("row {row} has invalid currency `{value}`")]
    UploadInvalidCurrency { row: usize, value: String },
    /// A CSV row contained an invalid price for a price level.
    #[error("row {row} has invalid price `{value}` for price level `{price_level}`")]
    UploadInvalidPrice {
        row: usize,
        price_level: String,
        value: String,
    },
    /// The form referenced a price level that does not exist.
    #[error("unknown price level id `{price_level_id}`")]
    UnknownPriceLevel { price_level_id: i32 },
    /// A provided price could not be parsed for the specified price level.
    #[error("invalid price `{value}` for price level `{price_level}`")]
    InvalidPriceLevelAmount { price_level: String, value: String },
    /// The uploaded CSV did not contain any usable products.
    #[error("upload contains no products")]
    EmptyUpload,
    /// CSV parsing failures.
    #[error("failed to parse CSV: {0}")]
    Csv(#[from] csv::Error),
    /// File system failures while reading the uploaded payload.
    #[error("failed to read uploaded file: {0}")]
    FileRead(#[from] std::io::Error),
    /// The provided category identifier could not be parsed.
    #[error("invalid category id `{value}`")]
    InvalidCategoryId { value: String },
}

/// Form payload emitted when submitting the "Add product" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddProductForm {
    /// Name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Optional SKU supplied by the user.
    #[validate(length(max = SKU_MAX_LEN_VALIDATOR))]
    pub sku: Option<String>,
    /// Optional longer description.
    pub description: Option<String>,
    /// Optional unit of measure.
    #[validate(length(max = UNITS_MAX_LEN_VALIDATOR))]
    pub units: Option<String>,
    /// ISO 4217 currency code (e.g. `USD`).
    #[validate(length(equal = CURRENCY_CODE_LEN_VALIDATOR))]
    pub currency: String,
    /// Optional category identifier selected by the user.
    #[validate(range(min = 1))]
    #[serde(default)]
    #[serde(deserialize_with = "empty_id_as_none")]
    pub category_id: Option<i32>,
    /// Optional set of tag identifiers selected by the user.
    #[serde(default)]
    pub tag_ids: Vec<i32>,
    /// Optional newline-separated image URLs.
    #[serde(default)]
    pub image_urls: Option<String>,
    /// Optional price level amounts submitted with the product.
    #[serde(default)]
    pub price_levels: Vec<AddProductPriceLevelForm>,
    /// Optional amount per unit
    #[serde(default)]
    pub amount: Option<f32>,
}

/// Price level payload submitted alongside a product form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddProductPriceLevelForm {
    #[validate(range(min = 1))]
    pub price_level_id: i32,
    pub price: String,
}

impl AddProductForm {
    /// Validates and sanitizes the payload into a domain `NewProduct`.
    pub fn into_new_product(self, hub_id: i32) -> ProductFormResult<NewProduct> {
        let result = self.into_new_product_with_prices(hub_id, &[])?;
        Ok(result.product)
    }

    /// Validates and sanitizes the payload into a product and price level amounts.
    pub fn into_new_product_with_prices(
        self,
        hub_id: i32,
        price_levels: &[PriceLevel],
    ) -> ProductFormResult<NewProductUpload> {
        self.validate()?;

        let AddProductForm {
            name,
            sku,
            description,
            units,
            currency,
            category_id,
            tag_ids,
            image_urls,
            price_levels: price_level_entries,
            amount,
        } = self;

        let sanitized_name = sanitize_text(&name).ok_or(ProductFormError::EmptyName)?;

        let sanitized_sku = sku.as_deref().and_then(sanitize_text);

        let sanitized_description = description.as_deref().and_then(sanitize_text);

        let sanitized_units = units.as_deref().and_then(sanitize_text);

        let currency = sanitize_text(&currency)
            .ok_or(ProductFormError::InvalidCurrency)?
            .to_ascii_uppercase();

        let mut new_product = NewProduct::new(hub_id, sanitized_name, currency);

        if let Some(sku) = sanitized_sku {
            new_product = new_product.with_sku(sku);
        }

        if let Some(description) = sanitized_description {
            new_product = new_product.with_description(description);
        }

        if let Some(units) = sanitized_units {
            new_product = new_product.with_units(units);
        }

        if let Some(category_id) = category_id {
            new_product = new_product.with_category_id(category_id);
        }

        if let Some(amount) = amount {
            new_product = new_product.with_amount(amount);
        }

        let mut sanitized_tags: Vec<i32> = tag_ids;
        sanitized_tags.sort_unstable();
        sanitized_tags.dedup();

        let price_level_map: HashMap<i32, &PriceLevel> =
            price_levels.iter().map(|level| (level.id, level)).collect();

        let mut parsed_price_levels = Vec::new();
        for entry in price_level_entries {
            entry.validate()?;

            let raw_price = entry.price;
            let trimmed = raw_price.trim();
            if trimmed.is_empty() {
                continue;
            }

            let price_level = price_level_map.get(&entry.price_level_id).ok_or(
                ProductFormError::UnknownPriceLevel {
                    price_level_id: entry.price_level_id,
                },
            )?;

            let price_cents = parse_price_to_cents(trimmed).ok_or_else(|| {
                ProductFormError::InvalidPriceLevelAmount {
                    price_level: price_level.name.clone(),
                    value: raw_price.to_string(),
                }
            })?;

            parsed_price_levels.push(NewProductUploadPriceLevel {
                price_level_id: price_level.id,
                price_cents,
            });
        }

        Ok(NewProductUpload {
            product: new_product,
            price_levels: parsed_price_levels,
            image_urls: sanitize_image_urls(image_urls),
            tag_ids: sanitized_tags,
        })
    }
}

/// Multipart-backed upload payload for bulk product creation.
#[derive(MultipartForm)]
pub struct UploadProductsForm {
    #[multipart(limit = "10MB")]
    /// Uploaded CSV containing product data.
    pub csv: TempFile,
}

/// Sanitized product plus associated price levels parsed from an upload row.
#[derive(Debug, Clone)]
pub struct NewProductUpload {
    /// Product fields extracted from the CSV row.
    pub product: NewProduct,
    /// Optional price level amounts supplied for the product.
    pub price_levels: Vec<NewProductUploadPriceLevel>,
    /// Sanitized image URLs supplied for the product form.
    pub image_urls: Vec<String>,
    /// Sanitized tag identifiers submitted with the product form.
    pub tag_ids: Vec<i32>,
}

/// Price level entry parsed for a newly uploaded product.
#[derive(Debug, Clone)]
pub struct NewProductUploadPriceLevel {
    /// Identifier of the price level supplied in the CSV.
    pub price_level_id: i32,
    /// Price represented in the smallest currency unit (for example cents).
    pub price_cents: i32,
}

impl UploadProductsForm {
    /// Parse the uploaded CSV and convert it into product payloads with optional price levels.
    pub fn into_new_products(
        &mut self,
        hub_id: i32,
        price_levels: &[PriceLevel],
    ) -> ProductFormResult<Vec<NewProductUpload>> {
        self.csv.file.rewind()?;
        let reader_source = self.csv.file.as_file_mut();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(reader_source);

        let headers = reader.headers()?.clone();
        let header_indexes = locate_product_headers(&headers);

        let name_index = header_indexes
            .name_index
            .ok_or(ProductFormError::MissingRequiredHeaders)?;
        let currency_index = header_indexes
            .currency_index
            .ok_or(ProductFormError::MissingRequiredHeaders)?;

        let price_level_columns = locate_price_level_headers(&headers, price_levels);

        let mut products = Vec::new();
        let mut processed_rows = 0;

        for (index, row) in reader.records().enumerate() {
            processed_rows += 1;
            let row_number = index + 2; // account for header row
            let record = row?;

            let raw_name = record.get(name_index).unwrap_or("");
            let sanitized_name = sanitize_text(raw_name)
                .ok_or(ProductFormError::UploadMissingName { row: row_number })?;

            let currency_raw = record.get(currency_index).unwrap_or("").trim();
            if currency_raw.is_empty() {
                return Err(ProductFormError::UploadMissingCurrency { row: row_number });
            }

            let currency = sanitize_text(currency_raw)
                .ok_or(ProductFormError::UploadInvalidCurrency {
                    row: row_number,
                    value: currency_raw.to_string(),
                })?
                .to_ascii_uppercase();

            let amount = header_indexes
                .amount_index
                .and_then(|idx| record.get(idx))
                .and_then(|val| str::parse::<f32>(val).ok());

            let sku = header_indexes
                .sku_index
                .and_then(|idx| record.get(idx))
                .and_then(sanitize_text);

            let description = header_indexes
                .description_index
                .and_then(|idx| record.get(idx))
                .and_then(sanitize_text);

            let units = header_indexes
                .units_index
                .and_then(|idx| record.get(idx))
                .and_then(sanitize_text);

            let mut product = NewProduct::new(hub_id, sanitized_name, currency);

            if let Some(sku) = sku {
                product = product.with_sku(sku);
            }

            if let Some(description) = description {
                product = product.with_description(description);
            }

            if let Some(units) = units {
                product = product.with_units(units);
            }

            if let Some(amount) = amount {
                product = product.with_amount(amount);
            }

            let mut parsed_price_levels = Vec::new();
            for column in &price_level_columns {
                let value = record.get(column.index).unwrap_or("").trim();
                if value.is_empty() {
                    continue;
                }

                let price_cents = parse_price_to_cents(value).ok_or_else(|| {
                    ProductFormError::UploadInvalidPrice {
                        row: row_number,
                        price_level: column.price_level.name.clone(),
                        value: value.to_string(),
                    }
                })?;

                parsed_price_levels.push(NewProductUploadPriceLevel {
                    price_level_id: column.price_level.id,
                    price_cents,
                });
            }

            products.push(NewProductUpload {
                product,
                price_levels: parsed_price_levels,
                image_urls: Vec::new(),
                tag_ids: Vec::new(),
            });
        }

        if processed_rows == 0 || products.is_empty() {
            return Err(ProductFormError::EmptyUpload);
        }

        Ok(products)
    }
}

/// Form payload emitted when editing an existing product.
#[derive(Debug, Deserialize, Validate)]
pub struct EditProductForm {
    #[validate(range(min = 1))]
    pub product_id: i32,
    /// Optional new name.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Optional SKU update (empty string clears the existing SKU).
    #[validate(length(max = SKU_MAX_LEN_VALIDATOR))]
    pub sku: Option<String>,
    /// Optional description update (empty string clears the existing description).
    pub description: Option<String>,
    /// Optional units update (empty string clears the existing units).
    #[validate(length(max = UNITS_MAX_LEN_VALIDATOR))]
    pub units: Option<String>,
    /// Optional currency update.
    #[validate(length(max = CURRENCY_CODE_LEN_VALIDATOR))]
    pub currency: String,
    /// Optional newline-separated image URLs.
    #[serde(default)]
    pub image_urls: Option<String>,
    /// Optional archive flag toggle.
    #[serde(default)]
    pub is_archived: bool,
    /// Optional category update (negative or zero clears the category).
    #[serde(default)]
    #[serde(deserialize_with = "empty_id_as_none")]
    pub category_id: Option<i32>,
    /// Optional set of tags to associate with the product.
    #[serde(default)]
    pub tag_ids: Vec<i32>,
    /// Optional amount per unit
    #[serde(default)]
    pub amount: Option<f32>,
}

/// Sanitized update payload returned when editing a product.
#[derive(Debug)]
pub struct EditProductUpdate {
    /// Core product update fields.
    pub product: UpdateProduct,
    /// Sanitized list of tag identifiers to assign.
    pub tag_ids: Vec<i32>,
    /// Sanitized image URLs submitted with the update.
    pub image_urls: Vec<String>,
}

impl EditProductForm {
    /// Validates and sanitizes the payload into a domain `UpdateProduct` alongside tag updates.
    pub fn into_update_product(self) -> ProductFormResult<EditProductUpdate> {
        self.validate()?;

        let EditProductForm {
            product_id: _,
            name,
            sku,
            description,
            units,
            currency,
            image_urls,
            is_archived,
            category_id,
            tag_ids,
            amount,
        } = self;

        let sanitized_name = sanitize_text(&name).ok_or(ProductFormError::EmptyName)?;

        let currency = sanitize_text(&currency)
            .ok_or(ProductFormError::InvalidCurrency)?
            .to_ascii_uppercase();

        let mut updates = UpdateProduct::new(sanitized_name, currency, is_archived);

        if let Some(sku) = sku
            && let Some(sanitized) = sanitize_text(&sku)
        {
            updates = updates.with_sku(sanitized);
        }

        if let Some(description) = description
            && let Some(sanitized) = sanitize_text(&description)
        {
            updates = updates.with_description(sanitized);
        }

        if let Some(units) = units
            && let Some(sanitized) = sanitize_text(&units)
        {
            updates = updates.with_units(sanitized);
        }

        if let Some(category_raw) = category_id {
            updates = updates.with_category_id(category_raw);
        }

        if let Some(amount) = amount {
            updates = updates.with_amount(amount);
        }

        let image_urls = sanitize_image_urls(image_urls);

        let mut sanitized_tags: Vec<i32> = tag_ids;
        sanitized_tags.sort_unstable();
        sanitized_tags.dedup();

        Ok(EditProductUpdate {
            product: updates,
            tag_ids: sanitized_tags,
            image_urls,
        })
    }
}

struct ProductHeaderIndexes {
    name_index: Option<usize>,
    sku_index: Option<usize>,
    description_index: Option<usize>,
    units_index: Option<usize>,
    currency_index: Option<usize>,
    amount_index: Option<usize>,
}

fn locate_product_headers(headers: &StringRecord) -> ProductHeaderIndexes {
    ProductHeaderIndexes {
        name_index: locate_header(headers, "name"),
        sku_index: locate_header(headers, "sku"),
        description_index: locate_header(headers, "description"),
        units_index: locate_header(headers, "units"),
        currency_index: locate_header(headers, "currency"),
        amount_index: locate_header(headers, "amount"),
    }
}

fn locate_header(headers: &StringRecord, expected: &str) -> Option<usize> {
    headers
        .iter()
        .position(|header| header.eq_ignore_ascii_case(expected))
}

struct PriceLevelColumn<'a> {
    price_level: &'a PriceLevel,
    index: usize,
}

fn locate_price_level_headers<'a>(
    headers: &StringRecord,
    price_levels: &'a [PriceLevel],
) -> Vec<PriceLevelColumn<'a>> {
    price_levels
        .iter()
        .filter_map(|price_level| {
            locate_header(headers, price_level.name.as_str())
                .map(|index| PriceLevelColumn { price_level, index })
        })
        .collect()
}

fn parse_price_to_cents(input: &str) -> Option<i32> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut normalized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            normalized.push(ch);
        } else if ch.is_whitespace() {
            continue;
        } else {
            return None;
        }
    }

    let normalized = normalized.replace(',', ".");
    let mut parts = normalized.split('.');
    let whole_part = parts.next()?;
    if whole_part.is_empty() || !whole_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let mut cents = whole_part.parse::<i64>().ok()?.checked_mul(100)?;

    if let Some(frac_part) = parts.next() {
        if parts.next().is_some() {
            return None;
        }

        if frac_part.is_empty() || !frac_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let mut buffer = frac_part.to_string();
        if buffer.len() == 1 {
            buffer.push('0');
        } else if buffer.len() > 2 {
            return None;
        }

        let fractional = buffer.parse::<i64>().ok()?;
        cents = cents.checked_add(fractional)?;
    }

    i32::try_from(cents).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    use actix_multipart::form::tempfile::TempFile;
    use tempfile::NamedTempFile;

    use crate::domain::price_level::PriceLevel;

    #[test]
    fn add_product_form_converts_successfully() {
        let form = AddProductForm {
            name: "  Deluxe  Product  ".to_string(),
            sku: Some(" sku-001 ".to_string()),
            description: Some(" First line.\n\n Second line.  ".to_string()),
            units: Some("  Box  ".to_string()),
            currency: "usd".to_string(),
            category_id: Some(7),
            tag_ids: vec![5, 7, 5],
            image_urls: Some(
                " https://example.com/one.png \n\nhttps://example.com/two.png  ".to_string(),
            ),
            price_levels: vec![
                AddProductPriceLevelForm {
                    price_level_id: 1,
                    price: "12.34".to_string(),
                },
                AddProductPriceLevelForm {
                    price_level_id: 2,
                    price: "  ".to_string(),
                },
            ],
            amount: None,
        };
        let price_levels = vec![
            build_price_level(1, "Retail"),
            build_price_level(2, "Wholesale"),
        ];

        let payload = form
            .into_new_product_with_prices(42, &price_levels)
            .expect("expected success");

        assert_eq!(payload.product.hub_id, 42);
        assert_eq!(payload.product.name, "Deluxe  Product");
        assert_eq!(payload.product.sku.as_deref(), Some("sku-001"));
        assert_eq!(
            payload.product.description.as_deref(),
            Some("First line.\n\n Second line.")
        );
        assert_eq!(payload.product.units.as_deref(), Some("Box"));
        assert_eq!(payload.product.currency, "USD");
        assert_eq!(payload.product.category_id, Some(7));
        assert_eq!(payload.price_levels.len(), 1);
        assert_eq!(payload.price_levels[0].price_level_id, 1);
        assert_eq!(payload.price_levels[0].price_cents, 1234);
        assert_eq!(payload.tag_ids, vec![5, 7]);
        assert_eq!(
            payload.image_urls,
            vec![
                "https://example.com/one.png".to_string(),
                "https://example.com/two.png".to_string()
            ]
        );
    }

    #[test]
    fn add_product_form_rejects_empty_name() {
        let form = AddProductForm {
            name: "   ".to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "USD".to_string(),
            category_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: Vec::new(),
            amount: None,
        };

        let result = form.into_new_product(1);

        assert!(matches!(result, Err(ProductFormError::EmptyName)));
    }

    #[test]
    fn add_product_form_rejects_invalid_currency() {
        let form = AddProductForm {
            name: "Widget".to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "   ".to_string(),
            category_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: Vec::new(),
            amount: None,
        };

        let result = form.into_new_product(1);

        assert!(matches!(result, Err(ProductFormError::InvalidCurrency)));
    }

    #[test]
    fn add_product_form_rejects_invalid_price_amount() {
        let form = AddProductForm {
            name: "Widget".to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "USD".to_string(),
            category_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: vec![AddProductPriceLevelForm {
                price_level_id: 1,
                price: "oops".to_string(),
            }],
            amount: None,
        };
        let levels = vec![build_price_level(1, "Retail")];

        let result = form.into_new_product_with_prices(1, &levels);

        assert!(matches!(
            result,
            Err(ProductFormError::InvalidPriceLevelAmount { price_level, value })
                if price_level == "Retail" && value == "oops"
        ));
    }

    #[test]
    fn add_product_form_rejects_unknown_price_level() {
        let form = AddProductForm {
            name: "Widget".to_string(),
            sku: None,
            description: None,
            units: None,
            currency: "USD".to_string(),
            category_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: vec![AddProductPriceLevelForm {
                price_level_id: 999,
                price: "10".to_string(),
            }],
            amount: None,
        };
        let levels = vec![build_price_level(1, "Retail")];

        let result = form.into_new_product_with_prices(1, &levels);

        assert!(matches!(
            result,
            Err(ProductFormError::UnknownPriceLevel { price_level_id }) if price_level_id == 999
        ));
    }

    #[test]
    fn upload_products_form_converts_rows() {
        let csv = "\
name,currency,sku,description,units,Retail,Wholesale
Apple,usd,APL-1,Fresh apple, Each ,12.34,9.99
Banana,usd,,Ripe banana,,8.50,
";
        let mut form = build_upload_form(csv);
        let price_levels = vec![
            build_price_level(1, "Retail"),
            build_price_level(2, "Wholesale"),
        ];

        let products = form
            .into_new_products(5, &price_levels)
            .expect("expected upload to succeed");

        assert_eq!(products.len(), 2);

        let first = &products[0];
        assert_eq!(first.product.name, "Apple");
        assert_eq!(first.product.sku.as_deref(), Some("APL-1"));
        assert_eq!(first.product.units.as_deref(), Some("Each"));
        assert_eq!(first.product.currency, "USD");
        assert_eq!(first.price_levels.len(), 2);
        assert_eq!(first.price_levels[0].price_level_id, 1);
        assert_eq!(first.price_levels[0].price_cents, 1234);
        assert_eq!(first.price_levels[1].price_level_id, 2);
        assert_eq!(first.price_levels[1].price_cents, 999);

        let second = &products[1];
        assert_eq!(second.product.name, "Banana");
        assert!(second.product.sku.is_none());
        assert!(second.product.units.is_none());
        assert_eq!(second.product.currency, "USD");
        assert_eq!(second.price_levels.len(), 1);
        assert_eq!(second.price_levels[0].price_level_id, 1);
        assert_eq!(second.price_levels[0].price_cents, 850);
    }

    #[test]
    fn upload_products_form_rejects_missing_currency_header() {
        let csv = "name,sku\nApple,APL-1\n";
        let mut form = build_upload_form(csv);

        let result = form.into_new_products(5, &[]);

        assert!(matches!(
            result,
            Err(ProductFormError::MissingRequiredHeaders)
        ));
    }

    #[test]
    fn upload_products_form_rejects_missing_currency_value() {
        let csv = "name,currency\nApple,\n";
        let mut form = build_upload_form(csv);

        let result = form.into_new_products(5, &[]);

        assert!(matches!(
            result,
            Err(ProductFormError::UploadMissingCurrency { row: 2 })
        ));
    }

    #[test]
    fn upload_products_form_rejects_invalid_price_value() {
        let csv = "name,currency,Retail\nApple,usd,not-a-price\n";
        let mut form = build_upload_form(csv);
        let price_levels = vec![build_price_level(42, "Retail")];

        let result = form.into_new_products(1, &price_levels);

        assert!(matches!(
            result,
            Err(ProductFormError::UploadInvalidPrice {
                row: 2,
                price_level,
                value
            }) if price_level == "Retail" && value == "not-a-price"
        ));
    }

    fn build_upload_form(csv: &str) -> UploadProductsForm {
        let mut file = NamedTempFile::new().expect("create temp file");
        file.write_all(csv.as_bytes()).expect("write csv contents");
        file.as_file_mut()
            .seek(SeekFrom::Start(0))
            .expect("rewind csv file");

        UploadProductsForm {
            csv: TempFile {
                file,
                content_type: None,
                file_name: Some("products.csv".to_string()),
                size: csv.len(),
            },
        }
    }

    fn build_price_level(id: i32, name: &str) -> PriceLevel {
        let epoch = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
            .expect("epoch timestamp")
            .naive_utc();

        PriceLevel {
            id,
            hub_id: 1,
            name: name.to_string(),
            created_at: epoch,
            updated_at: epoch,
            is_default: false,
        }
    }

    #[test]
    fn edit_product_form_converts_updates() {
        let form = EditProductForm {
            product_id: 1,
            name: "  Premium  Widget ".to_string(),
            sku: Some("  ".to_string()),
            description: Some(" Updated description. \n\n ".to_string()),
            units: Some("  ea ".to_string()),
            currency: "eur".to_string(),
            image_urls: Some(
                " https://example.com/alpha.jpg\n\nhttps://example.com/beta.jpg ".to_string(),
            ),
            is_archived: true,
            category_id: Some(12),
            tag_ids: vec![5, 7, 5],
            amount: None,
        };

        let payload = form.into_update_product().expect("expected success");
        let updates = payload.product;
        let tag_ids = payload.tag_ids;
        let image_urls = payload.image_urls;

        assert_eq!(updates.name.as_str(), "Premium  Widget");
        assert!(updates.sku.is_none());
        assert_eq!(updates.description.as_deref(), Some("Updated description."));
        assert_eq!(updates.units.as_deref(), Some("ea"));
        assert_eq!(updates.currency.as_str(), "EUR");
        assert!(updates.is_archived);
        assert_eq!(updates.category_id, Some(12));
        assert_eq!(tag_ids, vec![5, 7]);
        assert_eq!(
            image_urls,
            vec![
                "https://example.com/alpha.jpg".to_string(),
                "https://example.com/beta.jpg".to_string()
            ]
        );
    }

    #[test]
    fn edit_product_form_rejects_invalid_currency() {
        let form = EditProductForm {
            product_id: 1,
            name: "  Premium  Widget ".to_string(),
            sku: Some("  ".to_string()),
            description: Some(" Updated description. \n\n ".to_string()),
            units: Some("  ea ".to_string()),
            currency: "   ".to_string(),
            image_urls: None,
            is_archived: true,
            category_id: Some(12),
            tag_ids: vec![5, 7, 5],
            amount: None,
        };

        let result = form.into_update_product();

        assert!(matches!(result, Err(ProductFormError::InvalidCurrency)));
    }
}
