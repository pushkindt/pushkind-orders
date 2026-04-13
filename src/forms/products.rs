use std::{collections::HashMap, io::Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::{StringRecord, Trim};
use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::{Deserialize, Deserializer, de::Error as DeError};
use validator::Validate;

use crate::domain::{
    price_level::PriceLevel,
    product::{NewProduct, UpdateProduct},
    types::{
        CategoryId, CurrencyCode, HubId, ImageUrl, ProductAmount, ProductDescription, ProductName,
        ProductSku, ProductUnits, VendorId,
    },
};
use crate::forms::FormError;

fn sanitize_image_urls(input: Option<String>) -> Vec<ImageUrl> {
    input
        .as_deref()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| ImageUrl::new(line).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn optional_scalar_from_json_or_form<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                T::from_str(trimmed).map(Some).map_err(D::Error::custom)
            }
        }
        serde_json::Value::Number(number) => T::from_str(&number.to_string())
            .map(Some)
            .map_err(D::Error::custom),
        serde_json::Value::Bool(value) => T::from_str(if value { "true" } else { "false" })
            .map(Some)
            .map_err(D::Error::custom),
        other => Err(D::Error::custom(format!(
            "unsupported optional scalar value: {other}"
        ))),
    }
}

/// Result type returned by the product form helpers.
pub type ProductFormResult<T> = Result<T, FormError>;

/// Form payload emitted when submitting the "Add product" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddProductForm {
    /// Name entered by the user.
    #[validate(length(min = 1, message = "Название товара обязательно."))]
    pub name: String,
    /// Optional SKU supplied by the user.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub sku: Option<String>,
    /// Optional longer description.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub description: Option<String>,
    /// Optional unit of measure.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub units: Option<String>,
    /// ISO 4217 currency code (e.g. `USD`).
    #[validate(length(equal = 3, message = "Валюта должна состоять из 3 символов."))]
    pub currency: String,
    /// Optional category identifier selected by the user.
    #[validate(range(min = 1, message = "Категория указана неверно."))]
    #[serde(default)]
    #[serde(deserialize_with = "optional_scalar_from_json_or_form")]
    pub category_id: Option<i32>,
    /// Optional vendor identifier selected by the user.
    #[serde(default)]
    #[serde(deserialize_with = "optional_scalar_from_json_or_form")]
    pub vendor_id: Option<i32>,
    /// Optional set of tag identifiers selected by the user.
    #[serde(default)]
    pub tag_ids: Vec<i32>,
    /// Optional newline-separated image URLs.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub image_urls: Option<String>,
    /// Optional price level amounts submitted with the product.
    #[serde(default)]
    pub price_levels: Vec<AddProductPriceLevelForm>,
    /// Optional amount per unit
    #[serde(default)]
    #[serde(deserialize_with = "optional_scalar_from_json_or_form")]
    pub amount: Option<f32>,
}

/// Price level payload submitted alongside a product form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddProductPriceLevelForm {
    #[validate(range(min = 1, message = "Уровень цены указан неверно."))]
    pub price_level_id: i32,
    pub price: String,
}

impl<'a> TryFrom<(AddProductForm, i32, &'a [PriceLevel])> for AddProductPayload {
    type Error = FormError;

    fn try_from(value: (AddProductForm, i32, &'a [PriceLevel])) -> Result<Self, Self::Error> {
        let (form, hub_id, price_levels) = value;
        form.validate()?;

        let AddProductForm {
            name,
            sku,
            description,
            units,
            currency,
            category_id,
            vendor_id,
            tag_ids,
            image_urls,
            price_levels: price_level_entries,
            amount,
        } = form;

        let name = ProductName::new(name).map_err(|_| FormError::InvalidProductName)?;

        let sanitized_sku = sku
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let sanitized_units = units
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let currency = currency.trim();
        if currency.is_empty() {
            return Err(FormError::InvalidProductCurrency);
        }
        let currency = currency.to_ascii_uppercase();

        let currency =
            CurrencyCode::new(currency).map_err(|_| FormError::InvalidProductCurrency)?;

        let hub_id = HubId::new(hub_id).map_err(|_| FormError::InvalidProductCategoryId)?;

        let mut new_product = NewProduct::new(hub_id, name, currency);

        if let Some(sku) = sanitized_sku {
            let sku = ProductSku::new(sku).map_err(|_| FormError::InvalidProductSku)?;
            new_product = new_product.with_sku(sku);
        }

        if let Some(description) = description {
            let description = ProductDescription::new(&description)
                .map_err(|_| FormError::InvalidProductDescription)?;
            new_product = new_product.with_description(description);
        }

        if let Some(units) = sanitized_units {
            let units = ProductUnits::new(units).map_err(|_| FormError::InvalidProductUnits)?;
            new_product = new_product.with_units(units);
        }

        if let Some(category_id) = category_id {
            if category_id > 0 {
                let category_id = CategoryId::new(category_id)
                    .map_err(|_| FormError::InvalidProductCategoryId)?;
                new_product = new_product.with_category_id(category_id);
            } else if category_id < 0 {
                return Err(FormError::InvalidProductCategoryId);
            }
        }

        if let Some(vendor_id) = vendor_id {
            if vendor_id > 0 {
                let vendor_id =
                    VendorId::new(vendor_id).map_err(|_| FormError::InvalidProductVendorId)?;
                new_product = new_product.with_vendor_id(vendor_id);
            } else if vendor_id < 0 {
                return Err(FormError::InvalidProductVendorId);
            }
        }

        if let Some(amount) = amount {
            let amount = ProductAmount::new(amount).map_err(|_| FormError::InvalidProductAmount)?;
            new_product = new_product.with_amount(amount);
        }

        let mut sanitized_tags: Vec<i32> = tag_ids;
        sanitized_tags.sort_unstable();
        sanitized_tags.dedup();

        let price_level_map: HashMap<i32, &PriceLevel> = price_levels
            .iter()
            .map(|level| (level.id.get(), level))
            .collect();

        let mut parsed_price_levels = Vec::new();
        for (index, entry) in price_level_entries.into_iter().enumerate() {
            entry
                .validate()
                .map_err(|errors| FormError::PrefixedValidation {
                    prefix: format!("price_levels.{index}"),
                    errors,
                })?;

            let raw_price = entry.price;
            let trimmed = raw_price.trim();
            if trimmed.is_empty() {
                continue;
            }

            let price_level = price_level_map.get(&entry.price_level_id).ok_or(
                FormError::UnknownProductPriceLevel {
                    price_level_id: entry.price_level_id,
                },
            )?;

            let price_cents = parse_price_to_cents(trimmed).ok_or_else(|| {
                FormError::InvalidProductPriceLevelAmount {
                    price_level: price_level.name.as_str().to_string(),
                    value: raw_price.to_string(),
                }
            })?;

            parsed_price_levels.push(NewProductUploadPriceLevel {
                price_level_id: price_level.id.get(),
                price_cents,
            });
        }

        Ok(AddProductPayload {
            product: new_product,
            price_levels: parsed_price_levels,
            image_urls: sanitize_image_urls(image_urls),
            tag_ids: sanitized_tags,
            category: None,
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

#[derive(Debug)]
pub struct UploadProductsPayload {
    pub products: Vec<AddProductPayload>,
}

/// Sanitized product plus associated price levels parsed from an upload row.
#[derive(Debug, Clone)]
pub struct AddProductPayload {
    /// Product fields extracted from the CSV row.
    pub product: NewProduct,
    /// Optional price level amounts supplied for the product.
    pub price_levels: Vec<NewProductUploadPriceLevel>,
    /// Sanitized image URLs supplied for the product form.
    pub image_urls: Vec<ImageUrl>,
    /// Sanitized tag identifiers submitted with the product form.
    pub tag_ids: Vec<i32>,
    /// Optional category path for the product.
    pub category: Option<String>,
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
    ) -> ProductFormResult<Vec<AddProductPayload>> {
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
            .ok_or(FormError::MissingProductUploadHeaders)?;
        let currency_index = header_indexes
            .currency_index
            .ok_or(FormError::MissingProductUploadHeaders)?;

        let price_level_columns = locate_price_level_headers(&headers, price_levels);

        let mut products = Vec::new();
        let mut processed_rows = 0;

        for (index, row) in reader.records().enumerate() {
            processed_rows += 1;
            let row_number = index + 2; // account for header row
            let record = row?;

            let raw_name = record.get(name_index).unwrap_or("").trim();
            if raw_name.is_empty() {
                return Err(FormError::ProductUploadMissingName { row: row_number });
            }
            let sanitized_name = raw_name;

            let currency_raw = record.get(currency_index).unwrap_or("").trim();
            if currency_raw.is_empty() {
                return Err(FormError::ProductUploadMissingCurrency { row: row_number });
            }

            let currency = currency_raw.to_ascii_uppercase();

            let amount = header_indexes
                .amount_index
                .and_then(|idx| record.get(idx))
                .and_then(|val| str::parse::<f32>(val).ok());

            let sku = header_indexes
                .sku_index
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);

            let description = header_indexes
                .description_index
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);

            let units = header_indexes
                .units_index
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);

            let category = header_indexes
                .category_index
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);

            let hub_id = HubId::new(hub_id).map_err(|_| FormError::InvalidProductCategoryId)?;
            let name =
                ProductName::new(sanitized_name).map_err(|_| FormError::InvalidProductName)?;
            let currency = CurrencyCode::new(currency).map_err(|_| {
                FormError::ProductUploadInvalidCurrency {
                    row: row_number,
                    value: currency_raw.to_string(),
                }
            })?;

            let mut product = NewProduct::new(hub_id, name, currency);

            if let Some(sku) = sku {
                let sku = ProductSku::new(sku).map_err(|_| FormError::InvalidProductSku)?;
                product = product.with_sku(sku);
            }

            if let Some(description) = description {
                let description = ProductDescription::new(&description)
                    .map_err(|_| FormError::InvalidProductDescription)?;
                product = product.with_description(description);
            }

            if let Some(units) = units {
                let units = ProductUnits::new(units).map_err(|_| FormError::InvalidProductUnits)?;
                product = product.with_units(units);
            }

            if let Some(amount) = amount {
                let amount =
                    ProductAmount::new(amount).map_err(|_| FormError::InvalidProductAmount)?;
                product = product.with_amount(amount);
            }

            let mut parsed_price_levels = Vec::new();
            for column in &price_level_columns {
                let value = record.get(column.index).unwrap_or("").trim();
                if value.is_empty() {
                    continue;
                }

                let price_cents = parse_price_to_cents(value).ok_or_else(|| {
                    FormError::ProductUploadInvalidPrice {
                        row: row_number,
                        price_level: column.price_level.name.as_str().to_string(),
                        value: value.to_string(),
                    }
                })?;

                parsed_price_levels.push(NewProductUploadPriceLevel {
                    price_level_id: column.price_level.id.get(),
                    price_cents,
                });
            }

            products.push(AddProductPayload {
                product,
                price_levels: parsed_price_levels,
                image_urls: Vec::new(),
                tag_ids: Vec::new(),
                category,
            });
        }

        if processed_rows == 0 || products.is_empty() {
            return Err(FormError::EmptyProductUpload);
        }

        Ok(products)
    }
}

impl<'a> TryFrom<(UploadProductsForm, i32, &'a [PriceLevel])> for UploadProductsPayload {
    type Error = FormError;

    fn try_from(value: (UploadProductsForm, i32, &'a [PriceLevel])) -> Result<Self, Self::Error> {
        let (mut form, hub_id, price_levels) = value;
        let products = form.into_new_products(hub_id, price_levels)?;
        Ok(Self { products })
    }
}

/// Form payload emitted when editing an existing product.
#[derive(Debug, Deserialize, Validate)]
pub struct EditProductForm {
    #[validate(range(min = 1, message = "Идентификатор товара указан неверно."))]
    pub product_id: i32,
    /// Optional new name.
    #[validate(length(min = 1, message = "Название товара обязательно."))]
    pub name: String,
    /// Optional SKU update (empty string clears the existing SKU).
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub sku: Option<String>,
    /// Optional description update (empty string clears the existing description).
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub description: Option<String>,
    /// Optional units update (empty string clears the existing units).
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub units: Option<String>,
    /// Optional currency update.
    #[validate(length(equal = 3, message = "Валюта должна состоять из 3 символов."))]
    pub currency: String,
    /// Optional newline-separated image URLs.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub image_urls: Option<String>,
    /// Optional archive flag toggle.
    #[serde(default)]
    pub is_archived: bool,
    /// Optional category update (negative or zero clears the category).
    #[serde(default)]
    #[serde(deserialize_with = "optional_scalar_from_json_or_form")]
    pub category_id: Option<i32>,
    /// Optional vendor update (zero clears the vendor).
    #[serde(default)]
    #[serde(deserialize_with = "optional_scalar_from_json_or_form")]
    pub vendor_id: Option<i32>,
    /// Optional set of tags to associate with the product.
    #[serde(default)]
    pub tag_ids: Vec<i32>,
    /// Optional price level updates submitted with the form.
    #[serde(default)]
    pub price_levels: Vec<EditProductPriceLevelForm>,
    /// Optional amount per unit
    #[serde(default)]
    #[serde(deserialize_with = "optional_scalar_from_json_or_form")]
    pub amount: Option<f32>,
}

/// Price level payload submitted when editing a product.
#[derive(Debug, Deserialize, Validate)]
pub struct EditProductPriceLevelForm {
    #[validate(range(min = 1, message = "Уровень цены указан неверно."))]
    pub price_level_id: i32,
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub price: Option<String>,
}

/// Sanitized update payload returned when editing a product.
#[derive(Debug)]
pub struct EditProductPayload {
    /// Core product update fields.
    pub product: UpdateProduct,
    /// Sanitized list of tag identifiers to assign.
    pub tag_ids: Vec<i32>,
    /// Sanitized image URLs submitted with the update.
    pub image_urls: Vec<ImageUrl>,
    /// Sanitized price level assignments submitted with the update.
    pub price_levels: Vec<NewProductUploadPriceLevel>,
}

impl<'a> TryFrom<(EditProductForm, &'a [PriceLevel])> for EditProductPayload {
    type Error = FormError;

    fn try_from(value: (EditProductForm, &'a [PriceLevel])) -> Result<Self, Self::Error> {
        let (form, price_levels) = value;
        form.validate()?;

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
            vendor_id,
            tag_ids,
            price_levels: price_level_entries,
            amount,
        } = form;

        let name = ProductName::new(name).map_err(|_| FormError::InvalidProductName)?;

        let currency = currency.trim();
        if currency.is_empty() {
            return Err(FormError::InvalidProductCurrency);
        }
        let currency = currency.to_ascii_uppercase();

        let currency =
            CurrencyCode::new(currency).map_err(|_| FormError::InvalidProductCurrency)?;

        let mut updates = UpdateProduct::new(name, currency, is_archived);

        if let Some(sku) = sku {
            let trimmed = sku.trim();
            if !trimmed.is_empty() {
                let sku = ProductSku::new(trimmed).map_err(|_| FormError::InvalidProductSku)?;
                updates = updates.with_sku(sku);
            }
        }

        if let Some(description) = description {
            let description = ProductDescription::new(&description)
                .map_err(|_| FormError::InvalidProductDescription)?;
            updates = updates.with_description(description);
        }

        if let Some(units) = units {
            let trimmed = units.trim();
            if !trimmed.is_empty() {
                let units =
                    ProductUnits::new(trimmed).map_err(|_| FormError::InvalidProductUnits)?;
                updates = updates.with_units(units);
            }
        }

        if let Some(category_raw) = category_id {
            if category_raw > 0 {
                let category_id = CategoryId::new(category_raw)
                    .map_err(|_| FormError::InvalidProductCategoryId)?;
                updates = updates.with_category_id(category_id);
            } else if category_raw < 0 {
                return Err(FormError::InvalidProductCategoryId);
            }
        }

        if let Some(vendor_raw) = vendor_id {
            if vendor_raw > 0 {
                let vendor_id =
                    VendorId::new(vendor_raw).map_err(|_| FormError::InvalidProductVendorId)?;
                updates = updates.with_vendor_id(vendor_id);
            } else if vendor_raw == 0 {
                updates = updates.clear_vendor();
            } else {
                return Err(FormError::InvalidProductVendorId);
            }
        }

        if let Some(amount) = amount {
            let amount = ProductAmount::new(amount).map_err(|_| FormError::InvalidProductAmount)?;
            updates = updates.with_amount(amount);
        }

        let image_urls = sanitize_image_urls(image_urls);

        let mut sanitized_tags: Vec<i32> = tag_ids;
        sanitized_tags.sort_unstable();
        sanitized_tags.dedup();

        let price_level_map: HashMap<i32, &PriceLevel> = price_levels
            .iter()
            .map(|level| (level.id.get(), level))
            .collect();
        let mut parsed_price_levels = Vec::new();
        for (index, entry) in price_level_entries.into_iter().enumerate() {
            entry
                .validate()
                .map_err(|errors| FormError::PrefixedValidation {
                    prefix: format!("price_levels.{index}"),
                    errors,
                })?;

            let EditProductPriceLevelForm {
                price_level_id,
                price,
            } = entry;

            let raw_price = price.unwrap_or_default();
            let trimmed = raw_price.trim();
            if trimmed.is_empty() {
                continue;
            }

            let price_level = price_level_map
                .get(&price_level_id)
                .ok_or(FormError::UnknownProductPriceLevel { price_level_id })?;

            let price_cents = parse_price_to_cents(trimmed).ok_or_else(|| {
                FormError::InvalidProductPriceLevelAmount {
                    price_level: price_level.name.as_str().to_string(),
                    value: raw_price.to_string(),
                }
            })?;

            parsed_price_levels.push(NewProductUploadPriceLevel {
                price_level_id: price_level.id.get(),
                price_cents,
            });
        }

        Ok(EditProductPayload {
            product: updates,
            tag_ids: sanitized_tags,
            image_urls,
            price_levels: parsed_price_levels,
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
    category_index: Option<usize>,
}

fn locate_product_headers(headers: &StringRecord) -> ProductHeaderIndexes {
    ProductHeaderIndexes {
        name_index: locate_header(headers, "name"),
        sku_index: locate_header(headers, "sku"),
        description_index: locate_header(headers, "description"),
        units_index: locate_header(headers, "units"),
        currency_index: locate_header(headers, "currency"),
        amount_index: locate_header(headers, "amount"),
        category_index: locate_header(headers, "category"),
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
    use crate::domain::types::{HubId, PriceLevelId, PriceLevelName};

    #[test]
    fn add_product_form_converts_successfully() {
        let form = AddProductForm {
            name: "  Deluxe  Product  ".to_string(),
            sku: Some(" sku-001 ".to_string()),
            description: Some(" First line.\n\n Second line.  ".to_string()),
            units: Some("  Box  ".to_string()),
            currency: "usd".to_string(),
            category_id: Some(7),
            vendor_id: None,
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
        let price_levels = [
            build_price_level(1, "Retail"),
            build_price_level(2, "Wholesale"),
        ];

        let payload: AddProductPayload = (form, 42, &price_levels[..])
            .try_into()
            .expect("expected success");

        assert_eq!(payload.product.hub_id.get(), 42);
        assert_eq!(payload.product.name.as_str(), "Deluxe  Product");
        assert_eq!(
            payload.product.sku.as_ref().map(|sku| sku.as_str()),
            Some("sku-001")
        );
        assert_eq!(
            payload.product.description.as_ref().map(|d| d.as_str()),
            Some("First line.\n\n Second line.")
        );
        assert_eq!(
            payload.product.units.as_ref().map(|units| units.as_str()),
            Some("Box")
        );
        assert_eq!(payload.product.currency.as_str(), "USD");
        assert_eq!(payload.product.category_id.map(|id| id.get()), Some(7));
        assert_eq!(payload.price_levels.len(), 1);
        assert_eq!(payload.price_levels[0].price_level_id, 1);
        assert_eq!(payload.price_levels[0].price_cents, 1234);
        assert_eq!(payload.tag_ids, vec![5, 7]);
        assert_eq!(
            payload
                .image_urls
                .into_iter()
                .map(|url| url.into_inner())
                .collect::<Vec<_>>(),
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
            vendor_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: Vec::new(),
            amount: None,
        };

        let result: ProductFormResult<AddProductPayload> = (form, 1, &[][..]).try_into();

        assert!(matches!(result, Err(FormError::InvalidProductName)));
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
            vendor_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: Vec::new(),
            amount: None,
        };

        let result: ProductFormResult<AddProductPayload> = (form, 1, &[][..]).try_into();

        assert!(matches!(result, Err(FormError::InvalidProductCurrency)));
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
            vendor_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: vec![AddProductPriceLevelForm {
                price_level_id: 1,
                price: "oops".to_string(),
            }],
            amount: None,
        };
        let levels = [build_price_level(1, "Retail")];

        let result: ProductFormResult<AddProductPayload> = (form, 1, &levels[..]).try_into();

        assert!(matches!(
            result,
            Err(FormError::InvalidProductPriceLevelAmount { price_level, value })
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
            vendor_id: None,
            tag_ids: Vec::new(),
            image_urls: None,
            price_levels: vec![AddProductPriceLevelForm {
                price_level_id: 999,
                price: "10".to_string(),
            }],
            amount: None,
        };
        let levels = [build_price_level(1, "Retail")];

        let result: ProductFormResult<AddProductPayload> = (form, 1, &levels[..]).try_into();

        assert!(matches!(
            result,
            Err(FormError::UnknownProductPriceLevel { price_level_id }) if price_level_id == 999
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
        assert_eq!(first.product.name.as_str(), "Apple");
        assert_eq!(
            first.product.sku.as_ref().map(|sku| sku.as_str()),
            Some("APL-1")
        );
        assert_eq!(
            first.product.units.as_ref().map(|units| units.as_str()),
            Some("Each")
        );
        assert_eq!(first.product.currency.as_str(), "USD");
        assert_eq!(first.price_levels.len(), 2);
        assert_eq!(first.price_levels[0].price_level_id, 1);
        assert_eq!(first.price_levels[0].price_cents, 1234);
        assert_eq!(first.price_levels[1].price_level_id, 2);
        assert_eq!(first.price_levels[1].price_cents, 999);

        let second = &products[1];
        assert_eq!(second.product.name.as_str(), "Banana");
        assert!(second.product.sku.is_none());
        assert!(second.product.units.is_none());
        assert_eq!(second.product.currency.as_str(), "USD");
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
            Err(FormError::MissingProductUploadHeaders)
        ));
    }

    #[test]
    fn upload_products_form_rejects_missing_currency_value() {
        let csv = "name,currency\nApple,\n";
        let mut form = build_upload_form(csv);

        let result = form.into_new_products(5, &[]);

        assert!(matches!(
            result,
            Err(FormError::ProductUploadMissingCurrency { row: 2 })
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
            Err(FormError::ProductUploadInvalidPrice {
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
            id: PriceLevelId::new(id).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: PriceLevelName::new(name).unwrap(),
            created_at: epoch,
            updated_at: epoch,
            is_default: false,
        }
    }

    #[test]
    fn edit_product_form_converts_updates() {
        let price_levels = [
            build_price_level(1, "Retail"),
            build_price_level(2, "Wholesale"),
        ];
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
            vendor_id: None,
            tag_ids: vec![5, 7, 5],
            price_levels: vec![
                EditProductPriceLevelForm {
                    price_level_id: 1,
                    price: Some(" 12.50 ".to_string()),
                },
                EditProductPriceLevelForm {
                    price_level_id: 2,
                    price: Some("".to_string()),
                },
            ],
            amount: None,
        };

        let payload: EditProductPayload = (form, &price_levels[..])
            .try_into()
            .expect("expected success");
        let updates = payload.product;
        let tag_ids = payload.tag_ids;
        let image_urls = payload.image_urls;
        let price_updates = payload.price_levels;

        assert_eq!(updates.name.as_str(), "Premium  Widget");
        assert!(updates.sku.is_none());
        assert_eq!(
            updates.description.as_ref().map(|d| d.as_str()),
            Some("Updated description.")
        );
        assert_eq!(
            updates.units.as_ref().map(|units| units.as_str()),
            Some("ea")
        );
        assert_eq!(updates.currency.as_str(), "EUR");
        assert!(updates.is_archived);
        assert_eq!(updates.category_id.map(|id| id.get()), Some(12));
        assert_eq!(tag_ids, vec![5, 7]);
        assert_eq!(
            image_urls
                .into_iter()
                .map(|url| url.into_inner())
                .collect::<Vec<_>>(),
            vec![
                "https://example.com/alpha.jpg".to_string(),
                "https://example.com/beta.jpg".to_string()
            ]
        );
        assert_eq!(price_updates.len(), 1);
        assert_eq!(price_updates[0].price_level_id, 1);
        assert_eq!(price_updates[0].price_cents, 1250);
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
            vendor_id: None,
            tag_ids: vec![5, 7, 5],
            price_levels: Vec::new(),
            amount: None,
        };

        let result: ProductFormResult<EditProductPayload> = (form, &[][..]).try_into();

        assert!(matches!(result, Err(FormError::InvalidProductCurrency)));
    }
}
