use std::io::{Read, Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::Trim;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::domain::price_level::{NewPriceLevel, UpdatePriceLevel};

/// Maximum length allowed for a price level name.
const NAME_MAX_LEN: usize = 128;
const NAME_MAX_LEN_VALIDATOR: u64 = NAME_MAX_LEN as u64;

/// Result type returned by the price level form helpers.
pub type PriceLevelFormResult<T> = Result<T, PriceLevelFormError>;

/// Errors that can occur while processing price level forms.
#[derive(Debug, Error)]
pub enum PriceLevelFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The provided name is empty after sanitization.
    #[error("price level name cannot be empty")]
    EmptyName,
    /// The uploaded CSV is missing the required header fields.
    #[error("upload is missing a `name` column")]
    MissingRequiredHeaders,
    /// A row was missing the price level name.
    #[error("row {row} is missing a price level name")]
    UploadMissingName { row: usize },
    /// The upload did not contain any valid price levels.
    #[error("upload contains no price levels")]
    EmptyUpload,
    /// CSV parsing failures.
    #[error("failed to parse CSV: {0}")]
    Csv(#[from] csv::Error),
}

/// Form payload emitted when submitting the "Add price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddPriceLevelForm {
    /// Name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Is this a default price level?
    #[serde(default)]
    pub default: bool,
}

/// Payload emitted when assigning a price level to a client.
#[derive(Debug, Deserialize)]
pub struct AssignClientPriceLevelPayload {
    /// Hub identifier to scope the assignment.
    pub hub_id: i32,
    /// Customer email used as part of the composite key.
    pub email: String,
    /// Customer phone used as part of the composite key.
    #[serde(default)]
    pub phone: Option<String>,
    /// Selected price level identifier. `None` restores the default hub level.
    pub price_level_id: Option<i32>,
}

impl AssignClientPriceLevelPayload {
    /// Validates and normalizes the payload into an assignment request.
    pub fn into_assignment_request(self) -> PriceLevelFormResult<AssignClientPriceLevelInput> {
        let mut errors = ValidationErrors::new();

        if self.hub_id < 1 {
            errors.add("hub_id", ValidationError::new("invalid_hub_id"));
        }

        let normalized_email = self.email.trim().to_lowercase();
        if normalized_email.is_empty() {
            errors.add("email", ValidationError::new("empty_email"));
        }

        if let Some(id) = self.price_level_id {
            if id < 1 {
                errors.add(
                    "price_level_id",
                    ValidationError::new("invalid_price_level_id"),
                );
            }
        }

        if !errors.is_empty() {
            return Err(PriceLevelFormError::Validation(errors));
        }

        let normalized_phone = self.phone.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

        Ok(AssignClientPriceLevelInput {
            hub_id: self.hub_id,
            email: normalized_email,
            phone: normalized_phone,
            price_level_id: self.price_level_id,
        })
    }
}

/// Normalized payload that can be passed to the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignClientPriceLevelInput {
    pub hub_id: i32,
    pub email: String,
    pub phone: Option<String>,
    pub price_level_id: Option<i32>,
}

impl AddPriceLevelForm {
    /// Validates and sanitizes the payload into a domain `NewPriceLevel`.
    pub fn into_new_price_level(self, hub_id: i32) -> PriceLevelFormResult<NewPriceLevel> {
        self.validate()?;

        let sanitized_name = sanitize_plain_text(&self.name);
        if sanitized_name.is_empty() {
            return Err(PriceLevelFormError::EmptyName);
        }

        Ok(NewPriceLevel::new(hub_id, sanitized_name, self.default))
    }
}

/// Form payload emitted when submitting the "Edit price level" form.
#[derive(Debug, Deserialize, Validate)]
pub struct EditPriceLevelForm {
    /// Updated name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Updated default flag for the price level.
    #[serde(default)]
    pub default: bool,
}

impl EditPriceLevelForm {
    /// Validates and sanitizes the payload into a domain `UpdatePriceLevel`.
    pub fn into_update_price_level(self) -> PriceLevelFormResult<UpdatePriceLevel> {
        self.validate()?;

        let sanitized_name = sanitize_plain_text(&self.name);
        if sanitized_name.is_empty() {
            return Err(PriceLevelFormError::EmptyName);
        }

        Ok(UpdatePriceLevel::new(sanitized_name, self.default))
    }
}

#[derive(MultipartForm)]
/// Multipart form for uploading a CSV file with new price_levels.
pub struct UploadPriceLevelsForm {
    #[multipart(limit = "10MB")]
    /// Uploaded CSV file containing price_level data.
    pub csv: TempFile,
}

#[derive(Debug, Error)]
/// Errors that can occur while parsing an uploaded price_levels CSV file.
pub enum UploadPriceLevelsFormError {
    #[error("Error reading csv file")]
    FileReadError,
    #[error("Error parsing csv file")]
    CsvParseError,
}

impl From<std::io::Error> for UploadPriceLevelsFormError {
    fn from(_: std::io::Error) -> Self {
        UploadPriceLevelsFormError::FileReadError
    }
}

impl From<csv::Error> for UploadPriceLevelsFormError {
    fn from(_: csv::Error) -> Self {
        UploadPriceLevelsFormError::CsvParseError
    }
}

impl UploadPriceLevelsForm {
    /// Parse the uploaded CSV file into a list of [`NewPriceLevel`] records.
    pub fn into_new_price_levels(
        &mut self,
        hub_id: i32,
    ) -> Result<Vec<NewPriceLevel>, UploadPriceLevelsFormError> {
        self.csv.file.rewind()?;
        parse_price_levels(self.csv.file.by_ref(), hub_id)
    }
}

#[derive(Deserialize)]
struct PriceLevelCsvRow {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    name: Option<String>,
}

fn parse_price_levels<R: Read>(
    reader: R,
    hub_id: i32,
) -> Result<Vec<NewPriceLevel>, UploadPriceLevelsFormError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    let mut price_levels = Vec::new();

    for row in csv_reader.deserialize::<PriceLevelCsvRow>() {
        let record = row?;

        if let Some(name) = record.name {
            let price_level = NewPriceLevel::new(hub_id, name, false);

            price_levels.push(price_level);
        }
    }

    Ok(price_levels)
}

fn sanitize_plain_text(input: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    use actix_multipart::form::tempfile::TempFile;
    use tempfile::NamedTempFile;

    #[test]
    fn add_price_level_form_sanitizes_and_converts() {
        let form = AddPriceLevelForm {
            name: "  Premium\tLevel  ".to_string(),
            default: false,
        };

        let new_level = form.into_new_price_level(5).expect("expected success");

        assert_eq!(new_level.hub_id, 5);
        assert_eq!(new_level.name, "Premium Level");
    }

    #[test]
    fn assign_client_price_level_payload_validates_positive_ids() {
        let payload = AssignClientPriceLevelPayload {
            hub_id: 9,
            email: "USER@example.com".to_string(),
            phone: Some("  +1999  ".to_string()),
            price_level_id: Some(3),
        };

        let assignment = payload
            .into_assignment_request()
            .expect("expected valid payload");

        assert_eq!(assignment.hub_id, 9);
        assert_eq!(assignment.email, "user@example.com");
        assert_eq!(assignment.phone.as_deref(), Some("+1999"));
        assert_eq!(assignment.price_level_id, Some(3));
    }

    #[test]
    fn assign_client_price_level_payload_rejects_invalid_ids() {
        let payload = AssignClientPriceLevelPayload {
            hub_id: 0,
            email: "".to_string(),
            phone: None,
            price_level_id: Some(0),
        };

        let result = payload.into_assignment_request();

        assert!(result.is_err(), "expected validation error");
    }

    #[test]
    fn add_price_level_form_rejects_empty() {
        let form = AddPriceLevelForm {
            name: "   ".to_string(),
            default: false,
        };

        let result = form.into_new_price_level(1);

        assert!(matches!(result, Err(PriceLevelFormError::EmptyName)));
    }

    #[test]
    fn edit_price_level_form_sanitizes_and_converts() {
        let form = EditPriceLevelForm {
            name: "  Updated\nName  ".to_string(),
            default: true,
        };

        let update = form.into_update_price_level().expect("expected success");

        assert_eq!(update.name, "Updated Name");
        assert!(update.is_default);
    }

    #[test]
    fn edit_price_level_form_rejects_empty() {
        let form = EditPriceLevelForm {
            name: " \t".to_string(),
            default: false,
        };

        let result = form.into_update_price_level();

        assert!(matches!(result, Err(PriceLevelFormError::EmptyName)));
    }

    #[test]
    fn upload_form_converts_rows() {
        let mut form = build_upload_form("name\nSilver\nGold\n");

        let price_levels = form
            .into_new_price_levels(10)
            .expect("expected upload to succeed");

        assert_eq!(price_levels.len(), 2);
        assert_eq!(price_levels[0].name, "Silver");
        assert_eq!(price_levels[0].hub_id, 10);
    }

    #[test]
    fn upload_form_returns_empty_when_name_missing() {
        let mut form = build_upload_form("description\nfoo\n");

        let price_levels = form
            .into_new_price_levels(3)
            .expect("expected success despite missing header");

        assert!(price_levels.is_empty());
    }

    #[test]
    fn upload_form_allows_empty_body() {
        let mut form = build_upload_form("name\n");

        let price_levels = form
            .into_new_price_levels(3)
            .expect("expected empty but valid upload");

        assert!(price_levels.is_empty());
    }

    fn build_upload_form(csv: &str) -> UploadPriceLevelsForm {
        let mut file = NamedTempFile::new().expect("create temp file");
        file.write_all(csv.as_bytes()).expect("write csv file");
        file.as_file_mut()
            .seek(SeekFrom::Start(0))
            .expect("seek to start");

        UploadPriceLevelsForm {
            csv: TempFile {
                file,
                content_type: None,
                file_name: Some("levels.csv".to_string()),
                size: csv.len(),
            },
        }
    }
}
