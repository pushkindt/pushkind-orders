use std::io::Cursor;

use csv::{StringRecord, Trim};
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::product::{NewProduct, UpdateProduct};

/// Maximum allowed length for a product name.
const NAME_MAX_LEN: usize = 128;
const NAME_MAX_LEN_VALIDATOR: u64 = NAME_MAX_LEN as u64;

/// Maximum allowed length for a SKU.
const SKU_MAX_LEN: usize = 64;
const SKU_MAX_LEN_VALIDATOR: u64 = SKU_MAX_LEN as u64;

/// ISO 4217 currency codes are three ASCII alphabetic characters.
const CURRENCY_CODE_LEN: usize = 3;
const CURRENCY_CODE_LEN_VALIDATOR: u64 = CURRENCY_CODE_LEN as u64;

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
    #[error("invalid currency code `{value}`")]
    InvalidCurrency { value: String },
    /// The uploaded CSV is missing required columns.
    #[error("upload is missing the required `name`/`title` or `currency` headers")]
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
    /// The uploaded CSV did not contain any usable products.
    #[error("upload contains no products")]
    EmptyUpload,
    /// CSV parsing failures.
    #[error("failed to parse CSV: {0}")]
    Csv(#[from] csv::Error),
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
    /// ISO 4217 currency code (e.g. `USD`).
    #[validate(length(equal = CURRENCY_CODE_LEN_VALIDATOR))]
    pub currency: String,
}

impl AddProductForm {
    /// Validates and sanitizes the payload into a domain `NewProduct`.
    pub fn into_new_product(self, hub_id: i32) -> ProductFormResult<NewProduct> {
        self.validate()?;

        let sanitized_name = sanitize_inline_text(&self.name);
        if sanitized_name.is_empty() {
            return Err(ProductFormError::EmptyName);
        }

        let sanitized_sku = self
            .sku
            .as_deref()
            .map(sanitize_sku)
            .filter(|value| !value.is_empty());

        let sanitized_description = self
            .description
            .as_deref()
            .map(sanitize_multiline_text)
            .filter(|value| !value.is_empty());

        let currency = match sanitize_currency(&self.currency) {
            Ok(value) => value,
            Err(ProductFormError::InvalidCurrency { value }) => {
                return Err(ProductFormError::InvalidCurrency { value });
            }
            Err(other) => return Err(other),
        };

        let mut new_product = NewProduct::new(hub_id, sanitized_name, currency);

        if let Some(sku) = sanitized_sku {
            new_product = new_product.with_sku(sku);
        }

        if let Some(description) = sanitized_description {
            new_product = new_product.with_description(description);
        }

        Ok(new_product)
    }
}

/// Multipart-backed upload payload for bulk product creation.
#[derive(Debug)]
pub struct UploadProductsForm {
    /// Optional filename provided by the client.
    pub file_name: Option<String>,
    /// Raw CSV bytes received from the upload.
    pub bytes: Vec<u8>,
}

impl UploadProductsForm {
    /// Construct a new upload payload from the multipart data.
    pub fn new(file_name: Option<String>, bytes: Vec<u8>) -> Self {
        Self { file_name, bytes }
    }

    /// Parse the uploaded CSV and convert it into domain `NewProduct` values.
    pub fn into_new_products(self, hub_id: i32) -> ProductFormResult<Vec<NewProduct>> {
        let UploadProductsForm { bytes, .. } = self;
        let cursor = Cursor::new(bytes);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(cursor);

        let headers = reader.headers()?.clone();
        let header_indexes = locate_product_headers(&headers);

        if header_indexes.name_index.is_none() && header_indexes.title_index.is_none() {
            return Err(ProductFormError::MissingRequiredHeaders);
        }

        if header_indexes.currency_index.is_none() {
            return Err(ProductFormError::MissingRequiredHeaders);
        }

        let mut products = Vec::new();
        let mut processed_rows = 0;

        for (index, row) in reader.records().enumerate() {
            processed_rows += 1;
            let row_number = index + 2; // account for header row
            let record = row?;

            let resolved_name = resolve_product_name(
                &record,
                header_indexes.name_index,
                header_indexes.title_index,
            );
            let sanitized_name = sanitize_inline_text(resolved_name);
            if sanitized_name.is_empty() {
                return Err(ProductFormError::UploadMissingName { row: row_number });
            }

            let currency_raw = record
                .get(
                    header_indexes
                        .currency_index
                        .expect("currency index validated"),
                )
                .unwrap_or("")
                .trim();
            if currency_raw.is_empty() {
                return Err(ProductFormError::UploadMissingCurrency { row: row_number });
            }

            let currency = match sanitize_currency(currency_raw) {
                Ok(value) => value,
                Err(ProductFormError::InvalidCurrency { value }) => {
                    return Err(ProductFormError::UploadInvalidCurrency {
                        row: row_number,
                        value,
                    });
                }
                Err(other) => return Err(other),
            };

            let sku = header_indexes
                .sku_index
                .and_then(|idx| record.get(idx))
                .map(sanitize_sku)
                .filter(|value| !value.is_empty());

            let description = header_indexes
                .description_index
                .and_then(|idx| record.get(idx))
                .map(sanitize_multiline_text)
                .filter(|value| !value.is_empty());

            let mut product = NewProduct::new(hub_id, sanitized_name, currency);

            if let Some(sku) = sku {
                product = product.with_sku(sku);
            }

            if let Some(description) = description {
                product = product.with_description(description);
            }

            products.push(product);
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
    /// Optional new name.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: Option<String>,
    /// Optional SKU update (empty string clears the existing SKU).
    #[validate(length(max = SKU_MAX_LEN_VALIDATOR))]
    pub sku: Option<String>,
    /// Optional description update (empty string clears the existing description).
    pub description: Option<String>,
    /// Optional currency update.
    pub currency: Option<String>,
    /// Optional archive flag toggle.
    pub is_archived: Option<bool>,
}

impl EditProductForm {
    /// Validates and sanitizes the payload into a domain `UpdateProduct`.
    pub fn into_update_product(self) -> ProductFormResult<UpdateProduct> {
        self.validate()?;

        let mut updates = UpdateProduct::new();

        if let Some(name) = self.name {
            let sanitized = sanitize_inline_text(&name);
            if sanitized.is_empty() {
                return Err(ProductFormError::EmptyName);
            }
            updates = updates.name(sanitized);
        }

        if let Some(sku) = self.sku {
            let sanitized = sanitize_sku(&sku);
            if sanitized.is_empty() {
                updates = updates.sku(None::<String>);
            } else {
                updates = updates.sku(Some(sanitized));
            }
        }

        if let Some(description) = self.description {
            let sanitized = sanitize_multiline_text(&description);
            if sanitized.is_empty() {
                updates = updates.description(None::<String>);
            } else {
                updates = updates.description(Some(sanitized));
            }
        }

        if let Some(currency) = self.currency {
            let trimmed = currency.trim();
            if trimmed.is_empty() {
                return Err(ProductFormError::InvalidCurrency {
                    value: currency.to_string(),
                });
            }

            match sanitize_currency(trimmed) {
                Ok(value) => {
                    updates = updates.currency(value);
                }
                Err(ProductFormError::InvalidCurrency { value }) => {
                    return Err(ProductFormError::InvalidCurrency { value });
                }
                Err(other) => return Err(other),
            }
        }

        if let Some(is_archived) = self.is_archived {
            updates = updates.archived(is_archived);
        }

        Ok(updates)
    }
}

struct ProductHeaderIndexes {
    name_index: Option<usize>,
    title_index: Option<usize>,
    sku_index: Option<usize>,
    description_index: Option<usize>,
    currency_index: Option<usize>,
}

fn locate_product_headers(headers: &StringRecord) -> ProductHeaderIndexes {
    ProductHeaderIndexes {
        name_index: locate_header(headers, "name"),
        title_index: locate_header(headers, "title"),
        sku_index: locate_header(headers, "sku"),
        description_index: locate_header(headers, "description"),
        currency_index: locate_header(headers, "currency"),
    }
}

fn locate_header(headers: &StringRecord, expected: &str) -> Option<usize> {
    headers
        .iter()
        .position(|header| header.eq_ignore_ascii_case(expected))
}

fn resolve_product_name(
    record: &StringRecord,
    name_index: Option<usize>,
    title_index: Option<usize>,
) -> &str {
    if let Some(index) = name_index
        && let Some(value) = record.get(index)
        && !value.trim().is_empty()
    {
        return value;
    }

    if let Some(index) = title_index
        && let Some(value) = record.get(index)
        && !value.trim().is_empty()
    {
        return value;
    }

    ""
}

fn sanitize_inline_text(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    let mut previous_whitespace = false;

    for ch in input.trim().chars() {
        if ch.is_whitespace() {
            if !previous_whitespace {
                sanitized.push(' ');
                previous_whitespace = true;
            }
        } else if ch.is_control() {
            continue;
        } else {
            sanitized.push(ch);
            previous_whitespace = false;
        }
    }

    sanitized
}

fn sanitize_sku(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
}

fn sanitize_multiline_text(input: &str) -> String {
    let mut lines: Vec<String> = input.lines().map(sanitize_inline_text).collect();

    while matches!(lines.first(), Some(line) if line.is_empty()) {
        lines.remove(0);
    }

    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return String::new();
    }

    let mut result = Vec::with_capacity(lines.len());
    let mut previous_empty = false;
    for line in lines {
        let is_empty = line.is_empty();
        if is_empty {
            if previous_empty {
                continue;
            }
            previous_empty = true;
            result.push(String::new());
        } else {
            previous_empty = false;
            result.push(line);
        }
    }

    result.join("\n")
}

fn sanitize_currency(input: &str) -> ProductFormResult<String> {
    let trimmed = input.trim();
    if trimmed.len() != CURRENCY_CODE_LEN {
        return Err(ProductFormError::InvalidCurrency {
            value: trimmed.to_string(),
        });
    }

    if !trimmed.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return Err(ProductFormError::InvalidCurrency {
            value: trimmed.to_string(),
        });
    }

    Ok(trimmed.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_product_form_converts_successfully() {
        let form = AddProductForm {
            name: "  Deluxe  Product  ".to_string(),
            sku: Some(" sku-001 ".to_string()),
            description: Some(" First line.\n\n Second line.  ".to_string()),
            currency: "usd".to_string(),
        };

        let new_product = form.into_new_product(42).expect("expected success");

        assert_eq!(new_product.hub_id, 42);
        assert_eq!(new_product.name, "Deluxe Product");
        assert_eq!(new_product.sku.as_deref(), Some("sku-001"));
        assert_eq!(
            new_product.description.as_deref(),
            Some("First line.\n\nSecond line.")
        );
        assert_eq!(new_product.currency, "USD");
    }

    #[test]
    fn add_product_form_rejects_empty_name() {
        let form = AddProductForm {
            name: "   ".to_string(),
            sku: None,
            description: None,
            currency: "USD".to_string(),
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
            currency: "US!".to_string(),
        };

        let result = form.into_new_product(1);

        assert!(matches!(
            result,
            Err(ProductFormError::InvalidCurrency { value }) if value == "US!"
        ));
    }

    #[test]
    fn upload_products_form_converts_rows() {
        let csv = b"name,currency,sku,description\nApple,usd,APL-1,Fresh apple\nBanana,usd,,Ripe banana\n".to_vec();
        let form = UploadProductsForm::new(Some("products.csv".into()), csv);

        let products = form
            .into_new_products(5)
            .expect("expected upload to succeed");

        assert_eq!(products.len(), 2);
        assert_eq!(products[0].name, "Apple");
        assert_eq!(products[0].sku.as_deref(), Some("APL-1"));
        assert_eq!(products[0].currency, "USD");

        assert_eq!(products[1].name, "Banana");
        assert!(products[1].sku.is_none());
        assert_eq!(products[1].currency, "USD");
    }

    #[test]
    fn upload_products_form_rejects_missing_currency_header() {
        let csv = b"name,sku\nApple,APL-1\n".to_vec();
        let form = UploadProductsForm::new(None, csv);

        let result = form.into_new_products(5);

        assert!(matches!(
            result,
            Err(ProductFormError::MissingRequiredHeaders)
        ));
    }

    #[test]
    fn upload_products_form_rejects_missing_currency_value() {
        let csv = b"name,currency\nApple,\n".to_vec();
        let form = UploadProductsForm::new(None, csv);

        let result = form.into_new_products(5);

        assert!(matches!(
            result,
            Err(ProductFormError::UploadMissingCurrency { row: 2 })
        ));
    }

    #[test]
    fn edit_product_form_converts_updates() {
        let form = EditProductForm {
            name: Some("  Premium  Widget ".to_string()),
            sku: Some("  ".to_string()),
            description: Some(" Updated description. \n\n ".to_string()),
            currency: Some("eur".to_string()),
            is_archived: Some(true),
        };

        let updates = form.into_update_product().expect("expected success");

        assert_eq!(updates.name.as_deref(), Some("Premium Widget"));
        assert!(matches!(updates.sku, Some(None)));
        assert_eq!(
            updates
                .description
                .as_ref()
                .and_then(|value| value.as_deref()),
            Some("Updated description.")
        );
        assert_eq!(updates.currency.as_deref(), Some("EUR"));
        assert_eq!(updates.is_archived, Some(true));
    }

    #[test]
    fn edit_product_form_rejects_invalid_currency() {
        let form = EditProductForm {
            name: None,
            sku: None,
            description: None,
            currency: Some("1".to_string()),
            is_archived: None,
        };

        let result = form.into_update_product();

        assert!(matches!(
            result,
            Err(ProductFormError::InvalidCurrency { value }) if value == "1"
        ));
    }
}
