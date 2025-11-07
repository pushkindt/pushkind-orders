use std::fmt::Display;
use std::str::FromStr;

use phonenumber::{Mode, parse};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

pub mod categories;
pub mod main;
pub mod price_levels;
pub mod products;
pub mod store;
pub mod tags;

fn sanitize_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneNormalizationError {
    Empty,
    Invalid,
}

pub fn normalize_phone_to_e164(value: &str) -> Result<String, PhoneNormalizationError> {
    let sanitized = sanitize_text(value).ok_or(PhoneNormalizationError::Empty)?;
    let parsed = parse(None, &sanitized).map_err(|_| PhoneNormalizationError::Invalid)?;
    Ok(parsed.format().mode(Mode::E164).to_string())
}

pub fn empty_id_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    // Read as Option<String> first
    let opt: Option<String> = Option::deserialize(deserializer)?;

    match opt {
        None => Ok(None), // missing or null
        Some(s) => {
            let s = s.trim();
            if s.is_empty() {
                // empty string -> None
                Ok(None)
            } else {
                // non-empty string -> parse to T
                T::from_str(s).map(Some).map_err(D::Error::custom)
            }
        }
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

    #[test]
    fn normalize_phone_helper_formats_numbers() {
        let normalized =
            normalize_phone_to_e164("  +1 (555) 123-4567  ").expect("expected normalized phone");
        assert_eq!(normalized, "+15551234567");
    }

    #[test]
    fn normalize_phone_helper_rejects_invalid() {
        let result = normalize_phone_to_e164("not-a-phone");
        assert!(matches!(result, Err(PhoneNormalizationError::Invalid)));
    }
}
