use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::types::{CategoryDescription, CategoryId, CategoryName, ImageUrl};

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
    /// Invalid image URL format.
    #[error("invalid image URL format")]
    InvalidImageUrl,
    /// Invalid category parent id.
    #[error("invalid parent category id")]
    InvalidParentId,
    /// Invalid category description.
    #[error("invalid category description")]
    InvalidDescription,
}

/// Form payload emitted when submitting the "Add category" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddCategoryForm {
    /// Name entered by the user.
    #[validate(length(min = 1))]
    pub name: String,
    /// Optional description for the category.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub description: Option<String>,
    /// Optional parent category identifier in string form.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub parent_id: Option<i32>,
    /// Optional image URL for the category
    #[serde(default)]
    #[validate(url)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub image_url: Option<String>,
}

/// Normalized payload for creating a category.
#[derive(Debug, Clone)]
pub struct AddCategoryPayload {
    pub name: CategoryName,
    pub description: Option<CategoryDescription>,
    pub parent_id: Option<CategoryId>,
    pub image_url: Option<ImageUrl>,
}

impl TryFrom<AddCategoryForm> for AddCategoryPayload {
    type Error = CategoryFormError;

    fn try_from(value: AddCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let name = CategoryName::new(value.name).map_err(|_| CategoryFormError::EmptyName)?;

        let description = value
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(CategoryDescription::new)
            .transpose()
            .map_err(|_| CategoryFormError::InvalidDescription)?;

        let parent_id = value
            .parent_id
            .map(CategoryId::new)
            .transpose()
            .map_err(|_| CategoryFormError::InvalidParentId)?;

        let image_url = value
            .image_url
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(ImageUrl::new)
            .transpose()
            .map_err(|_| CategoryFormError::InvalidImageUrl)?;

        Ok(Self {
            name,
            description,
            parent_id,
            image_url,
        })
    }
}

/// Form payload emitted when editing an existing category.
#[derive(Debug, Deserialize, Validate)]
pub struct EditCategoryForm {
    /// Name submitted by the user.
    #[validate(length(min = 1))]
    pub name: String,
    /// Optional description update.
    #[serde(default)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub description: Option<String>,
    /// Optional archive toggle for the category.
    #[serde(default)]
    pub is_archived: bool,
    /// Optional image URL for the category
    #[serde(default)]
    #[validate(url)]
    #[serde(deserialize_with = "empty_string_as_none_fromstr")]
    pub image_url: Option<String>,
}

/// Normalized payload for updating a category.
#[derive(Debug, Clone)]
pub struct EditCategoryPayload {
    pub name: CategoryName,
    pub description: Option<CategoryDescription>,
    pub is_archived: bool,
    pub image_url: Option<ImageUrl>,
}

impl TryFrom<EditCategoryForm> for EditCategoryPayload {
    type Error = CategoryFormError;

    fn try_from(value: EditCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let name = CategoryName::new(value.name).map_err(|_| CategoryFormError::EmptyName)?;

        let description = value
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(CategoryDescription::new)
            .transpose()
            .map_err(|_| CategoryFormError::InvalidDescription)?;

        let image_url = value
            .image_url
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(ImageUrl::new)
            .transpose()
            .map_err(|_| CategoryFormError::InvalidImageUrl)?;

        Ok(Self {
            name,
            description,
            is_archived: value.is_archived,
            image_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_category_form_sanitizes_and_converts() {
        let form = AddCategoryForm {
            name: "  Fresh Produce  ".to_string(),
            description: Some("  Fruits\n\n Vegetables  ".to_string()),
            parent_id: Some(12),
            image_url: None,
        };

        let payload: AddCategoryPayload = form.try_into().expect("expected conversion to succeed");

        assert_eq!(payload.name.as_str(), "Fresh Produce");
        assert_eq!(
            payload.description.as_ref().map(|desc| desc.as_str()),
            Some("Fruits\n\n Vegetables")
        );
        assert_eq!(payload.parent_id.map(|id| id.get()), Some(12));
    }

    #[test]
    fn add_category_form_rejects_empty_name() {
        let form = AddCategoryForm {
            name: "   ".to_string(),
            description: None,
            parent_id: None,
            image_url: None,
        };

        let result: CategoryFormResult<AddCategoryPayload> = form.try_into();

        assert!(matches!(result, Err(CategoryFormError::EmptyName)));
    }

    #[test]
    fn add_category_form_rejects_invalid_parent_id() {
        let form = AddCategoryForm {
            name: "Pantry".to_string(),
            description: None,
            parent_id: Some(-1),
            image_url: None,
        };

        let result: CategoryFormResult<AddCategoryPayload> = form.try_into();
        assert!(matches!(result, Err(CategoryFormError::InvalidParentId)));
    }

    #[test]
    fn edit_category_form_builds_payload() {
        let form = EditCategoryForm {
            name: "  Pantry  ".to_string(),
            description: Some(" Dry goods ".to_string()),
            is_archived: true,
            image_url: None,
        };

        let update: EditCategoryPayload = form
            .try_into()
            .expect("expected payload conversion to succeed");

        assert_eq!(update.name.as_str(), "Pantry");
        assert_eq!(
            update.description.as_ref().map(|desc| desc.as_str()),
            Some("Dry goods")
        );
        assert!(update.is_archived);
    }

    #[test]
    fn edit_category_form_rejects_empty_name() {
        let form = EditCategoryForm {
            name: "   ".to_string(),
            description: None,
            is_archived: false,
            image_url: None,
        };

        let result: CategoryFormResult<EditCategoryPayload> = form.try_into();

        assert!(matches!(result, Err(CategoryFormError::EmptyName)));
    }

    #[test]
    fn edit_category_form_clears_parent_and_description() {
        let form = EditCategoryForm {
            name: " Pantry ".to_string(),
            description: Some("  ".to_string()),
            is_archived: false,
            image_url: None,
        };

        let update: EditCategoryPayload = form
            .try_into()
            .expect("expected payload conversion to succeed");

        assert!(update.description.is_none());
    }
}
