use std::borrow::Cow;

use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::types::{TagId, TagName};

/// Result type returned by the tag form helpers.
pub type TagFormResult<T> = Result<T, TagFormError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

/// Errors that can occur while processing tag forms.
#[derive(Debug, Error)]
pub enum TagFormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),
    #[error("Идентификатор тега указан неверно.")]
    InvalidTagId,
    #[error("Название тега указано неверно.")]
    InvalidTagName,
}

impl TagFormError {
    pub fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            Self::InvalidTagId => vec![field_error("tag_id", self.to_string())],
            Self::InvalidTagName => vec![field_error("name", self.to_string())],
        }
    }
}

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
    type Error = TagFormError;

    fn try_from(value: AddTagForm) -> Result<Self, Self::Error> {
        value.validate()?;

        TagName::new(value.name)
            .map(|name| Self { name })
            .map_err(|_| TagFormError::InvalidTagName)
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
    type Error = TagFormError;

    fn try_from(value: EditTagForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let tag_id = TagId::new(value.tag_id).map_err(|_| TagFormError::InvalidTagId)?;
        let name = TagName::new(value.name).map_err(|_| TagFormError::InvalidTagName)?;

        Ok(Self { tag_id, name })
    }
}

fn field_error(field: impl Into<Cow<'static, str>>, message: impl Into<String>) -> FormFieldError {
    FormFieldError {
        field: field.into(),
        message: Cow::Owned(message.into()),
    }
}

fn collect_validation_errors(errors: &ValidationErrors) -> Vec<FormFieldError> {
    let mut field_errors = Vec::new();

    for (field, errors) in errors.field_errors() {
        for error in errors {
            field_errors.push(field_error(
                match field.as_ref() {
                    "tag_id" => Cow::Borrowed("tag_id"),
                    "name" => Cow::Borrowed("name"),
                    other => Cow::Owned(other.to_string()),
                },
                error
                    .message
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "Некорректное значение.".to_string()),
            ));
        }
    }

    field_errors.sort_by(|left, right| left.field.cmp(&right.field));
    field_errors
}

fn validation_errors_display(errors: &ValidationErrors) -> String {
    let collected = collect_validation_errors(errors);

    if collected.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else if collected.len() == 1 {
        format!("Ошибка валидации формы: {}", collected[0].message)
    } else {
        format!(
            "Ошибка валидации формы: {}",
            collected
                .into_iter()
                .map(|error| error.message.into_owned())
                .collect::<Vec<_>>()
                .join("; ")
        )
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

        assert!(matches!(result, Err(TagFormError::InvalidTagName)));
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

        assert!(matches!(result, Err(TagFormError::InvalidTagName)));
    }
}
