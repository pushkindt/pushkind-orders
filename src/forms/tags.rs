use serde::Deserialize;
use validator::Validate;

use crate::domain::types::{TagId, TagName};
use crate::forms::FormError;

/// Result type returned by the tag form helpers.
pub type TagFormResult<T> = Result<T, FormError>;

/// Form payload emitted when submitting the "Add tag" form.
#[derive(Debug, Deserialize, Validate)]
pub struct AddTagForm {
    /// Name entered by the user.
    #[validate(length(min = 1, message = "Название тега обязательно."))]
    pub name: String,
}

/// Normalized payload for creating a tag.
#[derive(Debug, Clone)]
pub struct AddTagPayload {
    pub name: TagName,
}

impl TryFrom<AddTagForm> for AddTagPayload {
    type Error = FormError;

    fn try_from(value: AddTagForm) -> Result<Self, Self::Error> {
        value.validate()?;

        TagName::new(value.name)
            .map(|name| Self { name })
            .map_err(|_| FormError::InvalidTagName)
    }
}

/// Form payload emitted when editing an existing tag.
#[derive(Debug, Deserialize, Validate)]
pub struct EditTagForm {
    /// Identifier of the tag to update.
    #[validate(range(min = 1, message = "Идентификатор тега указан неверно."))]
    pub tag_id: i32,
    /// Updated name supplied by the user.
    #[validate(length(min = 1, message = "Название тега обязательно."))]
    pub name: String,
}

/// Normalized payload for updating a tag.
#[derive(Debug, Clone)]
pub struct EditTagPayload {
    pub tag_id: TagId,
    pub name: TagName,
}

impl TryFrom<EditTagForm> for EditTagPayload {
    type Error = FormError;

    fn try_from(value: EditTagForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let tag_id = TagId::new(value.tag_id).map_err(|_| FormError::InvalidTagId)?;
        let name = TagName::new(value.name).map_err(|_| FormError::InvalidTagName)?;

        Ok(Self { tag_id, name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_tag_form_sanitizes_and_converts() {
        let form = AddTagForm {
            name: "  Seasonal \t Specials  ".to_string(),
        };

        let payload: AddTagPayload = form.try_into().expect("expected conversion to succeed");

        assert_eq!(payload.name.as_str(), "Seasonal \t Specials");
    }

    #[test]
    fn add_tag_form_rejects_empty_name() {
        let form = AddTagForm {
            name: "   ".to_string(),
        };

        let result: TagFormResult<AddTagPayload> = form.try_into();

        assert!(matches!(result, Err(FormError::InvalidTagName)));
    }

    #[test]
    fn edit_tag_form_builds_update() {
        let form = EditTagForm {
            tag_id: 9,
            name: "  Limited\nEdition  ".to_string(),
        };

        let payload: EditTagPayload = form
            .try_into()
            .expect("expected payload conversion to succeed");

        assert_eq!(payload.tag_id.get(), 9);
        assert_eq!(payload.name.as_str(), "Limited\nEdition");
    }

    #[test]
    fn edit_tag_form_rejects_empty_name() {
        let form = EditTagForm {
            tag_id: 3,
            name: "  ".to_string(),
        };

        let result: TagFormResult<EditTagPayload> = form.try_into();

        assert!(matches!(result, Err(FormError::InvalidTagName)));
    }
}
