//! Form validation and request payload handling with serde and validator.

use std::borrow::Cow;

use thiserror::Error;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

pub mod categories;
pub mod main;
pub mod orders;
pub mod price_levels;
pub mod products;
pub mod store;
pub mod tags;
pub mod vendors;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

#[derive(Debug, Error)]
pub enum FormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),

    #[error("{}", prefixed_validation_errors_display(prefix, errors))]
    PrefixedValidation {
        prefix: String,
        errors: ValidationErrors,
    },

    #[error("Идентификатор заказа указан неверно.")]
    InvalidOrderId,
    #[error("Статус заказа указан неверно.")]
    InvalidOrderStatus,
    #[error("Внешний номер заказа указан неверно.")]
    InvalidOrderReference,
    #[error("Заметки заказа указаны неверно.")]
    InvalidOrderNotes,
    #[error("Адрес доставки указан неверно.")]
    InvalidOrderShippingAddress,
    #[error("Получатель указан неверно.")]
    InvalidOrderConsignee,
    #[error("Инструкции по доставке указаны неверно.")]
    InvalidOrderDeliveryNotes,
    #[error("Плательщик указан неверно.")]
    InvalidOrderPayer,
    #[error("Не выбраны позиции для обновления.")]
    EmptyOrderApprovals,
    #[error("Выберите товар.")]
    InvalidOrderApprovalProductId { index: usize },
    #[error("Количество должно быть положительным целым.")]
    InvalidOrderApprovalQuantity { index: usize },

    #[error("Идентификатор товара указан неверно.")]
    InvalidProductId,
    #[error("Название товара указано неверно.")]
    InvalidProductName,
    #[error("Описание товара указано неверно.")]
    InvalidProductDescription,
    #[error("Валюта товара указана неверно.")]
    InvalidProductCurrency,
    #[error("Артикул указан неверно.")]
    InvalidProductSku,
    #[error("Единица измерения указана неверно.")]
    InvalidProductUnits,
    #[error("Объём товара указан неверно.")]
    InvalidProductAmount,
    #[error("CSV-файл должен содержать столбцы name и currency.")]
    MissingProductUploadHeaders,
    #[error("В строке {row} не указано название товара.")]
    ProductUploadMissingName { row: usize },
    #[error("В строке {row} не указана валюта товара.")]
    ProductUploadMissingCurrency { row: usize },
    #[error("В строке {row} указана некорректная валюта: {value}.")]
    ProductUploadInvalidCurrency { row: usize, value: String },
    #[error("В строке {row} указана некорректная цена {value} для уровня «{price_level}».")]
    ProductUploadInvalidPrice {
        row: usize,
        price_level: String,
        value: String,
    },
    #[error("Уровень цены указан неверно.")]
    UnknownProductPriceLevel { price_level_id: i32 },
    #[error("Цена для уровня «{price_level}» указана неверно.")]
    InvalidProductPriceLevelAmount { price_level: String, value: String },
    #[error("CSV-файл не содержит товаров для загрузки.")]
    EmptyProductUpload,
    #[error("Не удалось прочитать CSV-файл: {0}")]
    Csv(#[from] csv::Error),
    #[error("Не удалось прочитать загруженный файл: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("Категория указана неверно.")]
    InvalidProductCategoryId,
    #[error("Поставщик указан неверно.")]
    InvalidProductVendorId,

    #[error("Название категории указано неверно.")]
    InvalidCategoryName,
    #[error("Ссылка на изображение указана неверно.")]
    InvalidCategoryImageUrl,
    #[error("Родительская категория указана неверно.")]
    InvalidCategoryParentId,
    #[error("Описание категории указано неверно.")]
    InvalidCategoryDescription,

    #[error("Идентификатор тега указан неверно.")]
    InvalidTagId,
    #[error("Название тега указано неверно.")]
    InvalidTagName,

    #[error("Название уровня цен указано неверно.")]
    InvalidPriceLevelName,
    #[error("Публичный идентификатор клиента указан неверно.")]
    InvalidClientPublicId,
    #[error("Имя клиента указано неверно.")]
    InvalidClientName,
    #[error("Телефон клиента указан неверно.")]
    InvalidClientPhone,
    #[error("Идентификатор уровня цен указан неверно.")]
    InvalidPriceLevelId,
    #[error("Базовый уровень цен указан неверно.")]
    InvalidBasePriceLevelId,
    #[error("Модификатор цены указан неверно.")]
    InvalidPriceModifier,
    #[error("Категория в исключениях указана неверно.")]
    InvalidExcludedCategoryId,
    #[error("Товар в исключениях указан неверно.")]
    InvalidExcludedProductId,
    #[error("Товар во включениях указан неверно.")]
    InvalidIncludedProductId,

    #[error("Идентификатор поставщика указан неверно.")]
    InvalidVendorId,
    #[error("Идентификатор пользователя указан неверно.")]
    InvalidUserId,
    #[error("Название поставщика указано неверно.")]
    InvalidVendorName,
    #[error("Имя пользователя указано неверно.")]
    InvalidUserName,
    #[error("Электронный адрес пользователя указан неверно.")]
    InvalidUserEmail,
}

impl FormError {
    pub fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            Self::PrefixedValidation { prefix, errors } => {
                collect_prefixed_validation_errors(prefix, errors)
            }
            Self::InvalidOrderId => vec![field_error("order_id", self.to_string())],
            Self::InvalidOrderStatus => vec![field_error("status", self.to_string())],
            Self::InvalidOrderReference => vec![field_error("reference", self.to_string())],
            Self::InvalidOrderNotes => vec![field_error("notes", self.to_string())],
            Self::InvalidOrderShippingAddress => {
                vec![field_error("shipping_address", self.to_string())]
            }
            Self::InvalidOrderConsignee => vec![field_error("consignee", self.to_string())],
            Self::InvalidOrderDeliveryNotes => {
                vec![field_error("delivery_notes", self.to_string())]
            }
            Self::InvalidOrderPayer => vec![field_error("payer", self.to_string())],
            Self::EmptyOrderApprovals => vec![field_error("approvals", self.to_string())],
            Self::InvalidOrderApprovalProductId { index } => vec![owned_field_error(
                format!("approvals.{index}.product_id"),
                self.to_string(),
            )],
            Self::InvalidOrderApprovalQuantity { index } => vec![owned_field_error(
                format!("approvals.{index}.approved_quantity"),
                self.to_string(),
            )],
            Self::InvalidProductId => vec![field_error("product_id", self.to_string())],
            Self::InvalidProductName => vec![field_error("name", self.to_string())],
            Self::InvalidProductDescription => {
                vec![field_error("description", self.to_string())]
            }
            Self::InvalidProductCurrency => vec![field_error("currency", self.to_string())],
            Self::InvalidProductSku => vec![field_error("sku", self.to_string())],
            Self::InvalidProductUnits => vec![field_error("units", self.to_string())],
            Self::InvalidProductAmount => vec![field_error("amount", self.to_string())],
            Self::MissingProductUploadHeaders
            | Self::ProductUploadMissingName { .. }
            | Self::ProductUploadMissingCurrency { .. }
            | Self::ProductUploadInvalidCurrency { .. }
            | Self::ProductUploadInvalidPrice { .. }
            | Self::EmptyProductUpload
            | Self::Csv(_)
            | Self::FileRead(_) => vec![field_error("csv", self.to_string())],
            Self::UnknownProductPriceLevel { .. } | Self::InvalidProductPriceLevelAmount { .. } => {
                vec![field_error("price_levels", self.to_string())]
            }
            Self::InvalidProductCategoryId => vec![field_error("category_id", self.to_string())],
            Self::InvalidProductVendorId => vec![field_error("vendor_id", self.to_string())],
            Self::InvalidCategoryName => vec![field_error("name", self.to_string())],
            Self::InvalidCategoryImageUrl => vec![field_error("image_url", self.to_string())],
            Self::InvalidCategoryParentId => vec![field_error("parent_id", self.to_string())],
            Self::InvalidCategoryDescription => {
                vec![field_error("description", self.to_string())]
            }
            Self::InvalidTagId => vec![field_error("tag_id", self.to_string())],
            Self::InvalidTagName => vec![field_error("name", self.to_string())],
            Self::InvalidPriceLevelName => vec![field_error("name", self.to_string())],
            Self::InvalidClientPublicId => vec![field_error("public_id", self.to_string())],
            Self::InvalidClientName => vec![field_error("name", self.to_string())],
            Self::InvalidClientPhone => vec![field_error("phone", self.to_string())],
            Self::InvalidPriceLevelId => vec![field_error("price_level_id", self.to_string())],
            Self::InvalidBasePriceLevelId => {
                vec![field_error("base_price_level_id", self.to_string())]
            }
            Self::InvalidPriceModifier => vec![field_error("price_modifier", self.to_string())],
            Self::InvalidExcludedCategoryId => {
                vec![field_error("excluded_category_ids", self.to_string())]
            }
            Self::InvalidExcludedProductId => {
                vec![field_error("excluded_product_ids", self.to_string())]
            }
            Self::InvalidIncludedProductId => {
                vec![field_error("included_product_ids", self.to_string())]
            }
            Self::InvalidVendorId => vec![field_error("vendor_id", self.to_string())],
            Self::InvalidUserId => vec![field_error("user_id", self.to_string())],
            Self::InvalidVendorName => vec![field_error("name", self.to_string())],
            Self::InvalidUserName => vec![field_error("name", self.to_string())],
            Self::InvalidUserEmail => vec![field_error("email", self.to_string())],
        }
    }
}

pub(crate) fn sanitize_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub(crate) fn collect_validation_errors(errors: &ValidationErrors) -> Vec<FormFieldError> {
    let mut collected = Vec::new();
    collect_validation_errors_with_prefix(None, errors, &mut collected);
    collected.sort_by(|left, right| {
        left.field
            .cmp(&right.field)
            .then_with(|| left.message.cmp(&right.message))
    });
    collected
}

pub(crate) fn collect_prefixed_validation_errors(
    prefix: &str,
    errors: &ValidationErrors,
) -> Vec<FormFieldError> {
    let mut collected = Vec::new();
    collect_validation_errors_with_prefix(Some(prefix), errors, &mut collected);
    collected.sort_by(|left, right| {
        left.field
            .cmp(&right.field)
            .then_with(|| left.message.cmp(&right.message))
    });
    collected
}

fn collect_validation_errors_with_prefix(
    prefix: Option<&str>,
    errors: &ValidationErrors,
    collected: &mut Vec<FormFieldError>,
) {
    for (field, kind) in errors.errors() {
        match kind {
            ValidationErrorsKind::Field(field_errors) => {
                let field_name = match prefix {
                    Some(prefix) => format!("{prefix}.{field}"),
                    None => field.to_string(),
                };

                for error in field_errors {
                    collected.push(owned_field_error(
                        field_name.clone(),
                        validation_error_message(error),
                    ));
                }
            }
            ValidationErrorsKind::Struct(nested) => {
                let nested_prefix = match prefix {
                    Some(prefix) => format!("{prefix}.{field}"),
                    None => field.to_string(),
                };

                collect_validation_errors_with_prefix(Some(&nested_prefix), nested, collected);
            }
            ValidationErrorsKind::List(items) => {
                for (index, nested) in items {
                    let nested_prefix = match prefix {
                        Some(prefix) => format!("{prefix}.{field}.{index}"),
                        None => format!("{field}.{index}"),
                    };

                    collect_validation_errors_with_prefix(Some(&nested_prefix), nested, collected);
                }
            }
        }
    }
}

pub(crate) fn validation_error_message(error: &ValidationError) -> Cow<'static, str> {
    error
        .message
        .clone()
        .unwrap_or(Cow::Borrowed("Поле заполнено некорректно."))
}

pub(crate) fn validation_errors_display(errors: &ValidationErrors) -> String {
    let messages = collect_validation_errors(errors)
        .into_iter()
        .map(|error| error.message.into_owned())
        .collect::<Vec<_>>();

    if messages.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else {
        format!("Ошибка валидации формы: {}", messages.join("; "))
    }
}

pub(crate) fn prefixed_validation_errors_display(
    prefix: &str,
    errors: &ValidationErrors,
) -> String {
    let messages = collect_prefixed_validation_errors(prefix, errors)
        .into_iter()
        .map(|error| error.message.into_owned())
        .collect::<Vec<_>>();

    if messages.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else {
        format!("Ошибка валидации формы: {}", messages.join("; "))
    }
}

pub(crate) fn field_error(
    field: &'static str,
    message: impl Into<Cow<'static, str>>,
) -> FormFieldError {
    FormFieldError {
        field: Cow::Borrowed(field),
        message: message.into(),
    }
}

pub(crate) fn owned_field_error(
    field: impl Into<String>,
    message: impl Into<Cow<'static, str>>,
) -> FormFieldError {
    FormFieldError {
        field: Cow::Owned(field.into()),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_text() {
        assert_eq!(sanitize_text("   test   "), Some("test".to_string()));
        assert!(sanitize_text("").is_none());
        assert!(sanitize_text("    ").is_none());
    }
}
