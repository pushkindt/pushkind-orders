use chrono::NaiveDateTime;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::tag::{NewTag, UpdateTag};

/// Maximum allowed length for a tag name.
const NAME_MAX_LEN: usize = 128;
const NAME_MAX_LEN_VALIDATOR: u64 = NAME_MAX_LEN as u64;

/// Result type returned by the tag form helpers.
pub type TagFormResult<T> = Result<T, TagFormError>;

/// Errors that can occur while processing tag forms.
#[derive(Debug, Error)]
pub enum TagFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The provided name is empty after sanitization.
    #[error("tag name cannot be empty")]
    EmptyName,
}

/// Form payload emitted when submitting the "Add tag" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddTagForm {
    /// Name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
}

impl AddTagForm {
    /// Validates and sanitizes the payload into a domain `NewTag`.
    pub fn into_new_tag(self, hub_id: i32) -> TagFormResult<NewTag> {
        self.validate()?;

        let sanitized_name = sanitize_inline_text(&self.name);
        if sanitized_name.is_empty() {
            return Err(TagFormError::EmptyName);
        }

        Ok(NewTag::new(hub_id, sanitized_name))
    }
}

/// Form payload emitted when editing an existing tag.
#[derive(Debug, Deserialize, Validate)]
pub struct EditTagForm {
    /// Identifier of the tag to update.
    #[validate(range(min = 1))]
    pub tag_id: i32,
    /// Updated name supplied by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
}

impl EditTagForm {
    /// Validates and sanitizes the payload into a domain `UpdateTag`.
    pub fn into_update_tag(self, updated_at: NaiveDateTime) -> TagFormResult<UpdateTag> {
        self.validate()?;

        let sanitized_name = sanitize_inline_text(&self.name);
        if sanitized_name.is_empty() {
            return Err(TagFormError::EmptyName);
        }

        Ok(UpdateTag {
            name: sanitized_name,
            updated_at,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn add_tag_form_sanitizes_and_converts() {
        let form = AddTagForm {
            name: "  Seasonal \t Specials  ".to_string(),
        };

        let new_tag = form
            .into_new_tag(5)
            .expect("expected conversion to succeed");

        assert_eq!(new_tag.hub_id, 5);
        assert_eq!(new_tag.name, "Seasonal Specials");
    }

    #[test]
    fn add_tag_form_rejects_empty_name() {
        let form = AddTagForm {
            name: "   ".to_string(),
        };

        let result = form.into_new_tag(1);

        assert!(matches!(result, Err(TagFormError::EmptyName)));
    }

    #[test]
    fn edit_tag_form_builds_update() {
        let updated_at = NaiveDate::from_ymd_opt(2023, 11, 14)
            .and_then(|date| date.and_hms_opt(12, 0, 0))
            .expect("valid timestamp");
        let form = EditTagForm {
            tag_id: 9,
            name: "  Limited\nEdition  ".to_string(),
        };

        let tag_id = form.tag_id;
        let update = form
            .into_update_tag(updated_at)
            .expect("expected payload conversion to succeed");

        assert_eq!(tag_id, 9);
        assert_eq!(update.name, "Limited Edition");
        assert_eq!(update.updated_at, updated_at);
    }

    #[test]
    fn edit_tag_form_rejects_empty_name() {
        let form = EditTagForm {
            tag_id: 3,
            name: "  ".to_string(),
        };

        let updated_at = NaiveDate::from_ymd_opt(2023, 1, 1)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("valid timestamp");

        let result = form.into_update_tag(updated_at);

        assert!(matches!(result, Err(TagFormError::EmptyName)));
    }
}
