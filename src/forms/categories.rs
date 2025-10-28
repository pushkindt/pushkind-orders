use std::collections::HashSet;

use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::category::{NewCategory, UpdateCategory};

/// Maximum length allowed for a category name.
const NAME_MAX_LEN: usize = 128;
const NAME_MAX_LEN_VALIDATOR: u64 = NAME_MAX_LEN as u64;

/// Maximum length allowed for a category description.
const DESCRIPTION_MAX_LEN: usize = 2048;
const DESCRIPTION_MAX_LEN_VALIDATOR: u64 = DESCRIPTION_MAX_LEN as u64;

/// Result type returned by the category form helpers.
pub type CategoryFormResult<T> = Result<T, CategoryFormError>;

/// Errors that can occur while processing category forms.
#[derive(Debug, Error)]
pub enum CategoryFormError {
    /// Validation failures from the `validator` crate.
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    /// The provided name is empty after sanitization.
    #[error("category name cannot be empty")]
    EmptyName,
    /// Supplied identifier field could not be parsed.
    #[error("invalid {field} `{value}`")]
    InvalidIdentifier { field: &'static str, value: String },
}

/// Form payload emitted when submitting the "Add category" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddCategoryForm {
    /// Name entered by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Optional description for the category.
    #[validate(length(max = DESCRIPTION_MAX_LEN_VALIDATOR))]
    #[serde(default)]
    pub description: Option<String>,
    /// Optional parent category identifier in string form.
    #[serde(default)]
    pub parent_id: Option<String>,
}

impl AddCategoryForm {
    /// Validates and sanitizes the payload into a domain `NewCategory`.
    pub fn into_new_category(self, hub_id: i32) -> CategoryFormResult<NewCategory> {
        self.validate()?;

        let sanitized_name = sanitize_inline_text(&self.name);
        if sanitized_name.is_empty() {
            return Err(CategoryFormError::EmptyName);
        }

        let sanitized_description = self
            .description
            .as_deref()
            .map(sanitize_multiline_text)
            .filter(|value| !value.is_empty());

        let parent_id = parse_optional_i32(self.parent_id, "parent category")?;

        let mut new_category = NewCategory::new(hub_id, sanitized_name);
        if let Some(description) = sanitized_description {
            new_category = new_category.with_description(description);
        }
        if let Some(parent_id) = parent_id {
            new_category = new_category.with_parent_id(parent_id);
        }

        Ok(new_category)
    }
}

/// Normalized payload produced by the "Edit category" form.
#[derive(Debug)]
pub struct EditCategoryPayload {
    /// Identifier of the category to update.
    pub category_id: i32,
    /// Patch data that should be applied to the category.
    pub update: UpdateCategory,
}

/// Form payload emitted when editing an existing category.
#[derive(Debug, Deserialize, Validate)]
pub struct EditCategoryForm {
    /// Identifier of the category to update.
    #[validate(range(min = 1))]
    pub category_id: i32,
    /// Name submitted by the user.
    #[validate(length(min = 1, max = NAME_MAX_LEN_VALIDATOR))]
    pub name: String,
    /// Optional description update.
    #[validate(length(max = DESCRIPTION_MAX_LEN_VALIDATOR))]
    pub description: Option<String>,
    /// Optional archive toggle for the category.
    pub is_archived: bool,
}

impl EditCategoryForm {
    /// Validates and sanitizes the payload into a domain `UpdateCategory`.
    pub fn into_update_category(self) -> CategoryFormResult<EditCategoryPayload> {
        self.validate()?;

        let EditCategoryForm {
            category_id,
            name,
            description,
            is_archived,
        } = self;

        let name = {
            let sanitized = sanitize_inline_text(&name);
            if sanitized.is_empty() {
                return Err(CategoryFormError::EmptyName);
            }
            sanitized
        };

        let description = match description {
            Some(text) => {
                let sanitized = sanitize_multiline_text(&text);
                if sanitized.is_empty() {
                    None
                } else {
                    Some(sanitized)
                }
            }
            None => None,
        };

        let update = UpdateCategory::new(name, description, is_archived);

        Ok(EditCategoryPayload {
            category_id,
            update,
        })
    }
}

/// Normalized payload produced by the "Assign child categories" form.
#[derive(Debug)]
pub struct AssignChildCategoriesPayload {
    /// Identifier of the parent category.
    pub parent_id: i32,
    /// Unique list of child category identifiers to associate with the parent.
    pub child_ids: Vec<i32>,
}

/// Form payload emitted when submitting the "Assign child categories" form.
#[derive(Debug, Deserialize)]
pub struct AssignChildCategoriesForm {
    /// Identifier of the parent category.
    pub parent_id: i32,
    /// Identifiers of the child categories selected by the user.
    #[serde(default)]
    pub child_ids: Vec<i32>,
}

impl AssignChildCategoriesForm {
    /// Sanitizes the payload into a normalized assignment request.
    pub fn into_payload(self) -> AssignChildCategoriesPayload {
        let mut seen = HashSet::new();
        let mut normalized = Vec::new();

        for child_id in self.child_ids {
            if child_id <= 0 || child_id == self.parent_id {
                continue;
            }

            if seen.insert(child_id) {
                normalized.push(child_id);
            }
        }

        AssignChildCategoriesPayload {
            parent_id: self.parent_id,
            child_ids: normalized,
        }
    }
}

fn parse_optional_i32(
    value: Option<String>,
    field: &'static str,
) -> CategoryFormResult<Option<i32>> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                match trimmed.parse::<i32>() {
                    Ok(parsed) if parsed > 0 => Ok(Some(parsed)),
                    Ok(_) => Ok(None),
                    Err(_) => Err(CategoryFormError::InvalidIdentifier {
                        field,
                        value: trimmed.to_string(),
                    }),
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_category_form_sanitizes_and_converts() {
        let form = AddCategoryForm {
            name: "  Fresh Produce  ".to_string(),
            description: Some("  Fruits\n\n Vegetables  ".to_string()),
            parent_id: Some(" 12 ".to_string()),
        };

        let new_category = form
            .into_new_category(5)
            .expect("expected conversion to succeed");

        assert_eq!(new_category.hub_id, 5);
        assert_eq!(new_category.name, "Fresh Produce");
        assert_eq!(
            new_category.description.as_deref(),
            Some("Fruits\n\nVegetables")
        );
        assert_eq!(new_category.parent_id, Some(12));
    }

    #[test]
    fn add_category_form_rejects_empty_name() {
        let form = AddCategoryForm {
            name: "   ".to_string(),
            description: None,
            parent_id: None,
        };

        let result = form.into_new_category(1);

        assert!(matches!(result, Err(CategoryFormError::EmptyName)));
    }

    #[test]
    fn add_category_form_rejects_invalid_parent_id() {
        let form = AddCategoryForm {
            name: "Pantry".to_string(),
            description: None,
            parent_id: Some("abc".to_string()),
        };

        let result = form.into_new_category(1);

        assert!(matches!(
            result,
            Err(CategoryFormError::InvalidIdentifier { field, value })
                if field == "parent category" && value == "abc"
        ));
    }

    #[test]
    fn assign_child_categories_form_filters_duplicates_and_parent() {
        let form = AssignChildCategoriesForm {
            parent_id: 10,
            child_ids: vec![11, 10, 12, 11, -1],
        };

        let payload = form.into_payload();

        assert_eq!(payload.parent_id, 10);
        assert_eq!(payload.child_ids, vec![11, 12]);
    }

    #[test]
    fn edit_category_form_builds_payload() {
        let form = EditCategoryForm {
            category_id: 42,
            name: "  Pantry  ".to_string(),
            description: Some(" Dry goods ".to_string()),
            is_archived: true,
        };

        let payload = form
            .into_update_category()
            .expect("expected payload conversion to succeed");

        assert_eq!(payload.category_id, 42);
        let update = payload.update;
        assert_eq!(update.name, "Pantry");
        assert_eq!(update.description.as_deref(), Some("Dry goods"));
        assert!(update.is_archived);
    }

    #[test]
    fn edit_category_form_rejects_empty_name() {
        let form = EditCategoryForm {
            category_id: 1,
            name: "   ".to_string(),
            description: None,
            is_archived: false,
        };

        let result = form.into_update_category();

        assert!(matches!(result, Err(CategoryFormError::EmptyName)));
    }

    #[test]
    fn edit_category_form_clears_parent_and_description() {
        let form = EditCategoryForm {
            category_id: 2,
            name: " Pantry ".to_string(),
            description: Some("  ".to_string()),
            is_archived: false,
        };

        let payload = form
            .into_update_category()
            .expect("expected payload conversion to succeed");

        let update = payload.update;

        assert!(update.description.is_none());
    }
}
