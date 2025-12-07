//! Strongly-typed value objects used by domain entities.
//!
//! These wrappers enforce basic invariants (e.g., positive identifiers,
//! normalized/validated email) so that once a value reaches the domain layer it
//! can be treated as trusted.

use phonenumber::{Mode, parse};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use validator::ValidateEmail;

/// Errors produced when attempting to construct a constrained value object.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TypeConstraintError {
    /// Provided identifier is zero or negative.
    #[error("id must be greater than zero")]
    NonPositiveId,
    /// Provided price is zero or negative.
    #[error("price must be greater than zero")]
    NonPositivePrice,
    /// Provided amount is zero or negative.
    #[error("amount must be greater than zero")]
    NonPositiveAmount,
    /// Provided email failed format validation.
    #[error("invalid email address")]
    InvalidEmail,
    /// Provided string contained no non-whitespace characters.
    #[error("value cannot be empty")]
    EmptyString,
    /// Provided value failed custom validation.
    #[error("invalid value: {0}")]
    InvalidValue(String),
    /// Phone number did not meet expected format.
    #[error("invalid phone number")]
    InvalidPhone,
    /// OTP code is not six ASCII digits.
    #[error("invalid OTP code")]
    InvalidOtpCode,
    /// Order status string failed to parse to a valid enum.
    #[error("invalid order status")]
    InvalidOrderStatus,
}

/// Macro to generate lightweight newtypes for positive identifiers.
macro_rules! id_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(i32);

        impl $name {
            /// Creates a new identifier ensuring it is greater than zero.
            pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
                if value > 0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NonPositiveId)
                }
            }

            /// Returns the raw `i32` backing this identifier.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

id_newtype!(UserId, "Unique identifier for a user.");
id_newtype!(HubId, "Unique identifier for a hub.");
id_newtype!(TagId, "Unique identifier for a tag.");
id_newtype!(
    ProductTagId,
    "Unique identifier for a product-tag association."
);
id_newtype!(ProductId, "Unique identifier for a product.");
id_newtype!(CustomerId, "Unique identifier for a customer.");
id_newtype!(CategoryId, "Unique identifier for a category.");
id_newtype!(
    ProductPriceLevelRateId,
    "Unique identifier for a product price level rate."
);
id_newtype!(PriceLevelId, "Unique identifier for a price level.");
id_newtype!(OrderId, "Unique identifier for an order.");

/// Price stored in the smallest currency unit; must be positive.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PriceCents(i32);

impl PriceCents {
    /// Construct a new price ensuring it is above zero.
    pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
        if value > 0 {
            Ok(Self(value))
        } else {
            Err(TypeConstraintError::NonPositivePrice)
        }
    }

    /// Return the raw integer amount.
    pub const fn get(self) -> i32 {
        self.0
    }
}

impl Display for PriceCents {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i32> for PriceCents {
    type Error = TypeConstraintError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PriceCents> for i32 {
    fn from(value: PriceCents) -> Self {
        value.0
    }
}

/// Lower-cased and validated email address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserEmail(String);

impl UserEmail {
    /// Validates and normalizes an email string.
    pub fn new<S: Into<String>>(email: S) -> Result<Self, TypeConstraintError> {
        let normalized = email.into().trim().to_lowercase();
        if normalized.validate_email() {
            Ok(Self(normalized))
        } else {
            Err(TypeConstraintError::InvalidEmail)
        }
    }

    /// Borrow the email as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the owned inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for UserEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for UserEmail {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for UserEmail {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UserEmail> for String {
    fn from(value: UserEmail) -> Self {
        value.0
    }
}

/// Wrapper for non-empty, trimmed strings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Trims whitespace and rejects empty inputs.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let trimmed = value.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(TypeConstraintError::EmptyString);
        }
        Ok(Self(trimmed))
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper returning the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

macro_rules! non_empty_string_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            /// Constructs a trimmed, non-empty value.
            pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
                let inner = NonEmptyString::new(value)?;
                Ok(Self(inner.into_inner()))
            }

            /// Borrow the value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the wrapper and return the owned string.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

non_empty_string_newtype!(
    UserName,
    "Optional user name wrapper enforcing non-empty values."
);

non_empty_string_newtype!(TagName, "Tag name wrapper enforcing non-empty values.");

non_empty_string_newtype!(
    CustomerName,
    "Customer name wrapper enforcing non-empty values."
);

non_empty_string_newtype!(
    CategoryName,
    "Category name wrapper enforcing non-empty values."
);

non_empty_string_newtype!(
    ProductName,
    "Product name wrapper enforcing non-empty values."
);

impl std::ops::Deref for ProductName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for ProductName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for ProductName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<ProductName> for &str {
    fn eq(&self, other: &ProductName) -> bool {
        *self == other.as_str()
    }
}

non_empty_string_newtype!(
    PriceLevelName,
    "Price level name wrapper enforcing non-empty values."
);

non_empty_string_newtype!(
    ProductSku,
    "Optional SKU wrapper enforcing non-empty values."
);

impl std::ops::Deref for ProductSku {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for ProductSku {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for ProductSku {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<ProductSku> for &str {
    fn eq(&self, other: &ProductSku) -> bool {
        *self == other.as_str()
    }
}

non_empty_string_newtype!(ProductUnits, "Units wrapper enforcing non-empty values.");

impl std::ops::Deref for ProductUnits {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for ProductUnits {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for ProductUnits {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<ProductUnits> for &str {
    fn eq(&self, other: &ProductUnits) -> bool {
        *self == other.as_str()
    }
}

/// ISO 4217 currency code wrapper enforcing three uppercase ASCII letters.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Constructs a currency code that must be three uppercase ASCII letters.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let code = NonEmptyString::new(value)?;
        let value = code.as_str();
        if value.len() != 3 || !value.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(TypeConstraintError::InvalidValue(
                "currency code must be three uppercase letters".to_string(),
            ));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for CurrencyCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CurrencyCode {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for CurrencyCode {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CurrencyCode> for String {
    fn from(value: CurrencyCode) -> Self {
        value.0
    }
}

impl std::ops::Deref for CurrencyCode {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for CurrencyCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for CurrencyCode {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<CurrencyCode> for &str {
    fn eq(&self, other: &CurrencyCode) -> bool {
        *self == other.as_str()
    }
}

non_empty_string_newtype!(
    CategoryDescription,
    "Optional descriptive text for categories."
);

non_empty_string_newtype!(
    ProductDescription,
    "Optional descriptive text for products."
);

/// Product amount; must be positive.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct ProductAmount(f32);

impl ProductAmount {
    /// Construct a new amount ensuring it is above zero.
    pub fn new(value: f32) -> Result<Self, TypeConstraintError> {
        if value > 0.0 {
            Ok(Self(value))
        } else {
            Err(TypeConstraintError::NonPositiveAmount)
        }
    }

    /// Return the raw integer amount.
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Display for ProductAmount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<f32> for ProductAmount {
    type Error = TypeConstraintError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ProductAmount> for f32 {
    fn from(value: ProductAmount) -> Self {
        value.0
    }
}

non_empty_string_newtype!(ImageUrl, "Validated image URL wrapper.");

/// Normalizes a phone number string to E.164 format.
pub fn normalize_phone_to_e164(value: &str) -> Result<String, TypeConstraintError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TypeConstraintError::EmptyString);
    }
    let parsed = parse(None, trimmed).map_err(|_| TypeConstraintError::InvalidPhone)?;
    Ok(parsed.format().mode(Mode::E164).to_string())
}

/// Normalized phone number wrapper (expected E.164).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PhoneNumber(String);

impl PhoneNumber {
    /// Constructs a phone number ensuring it is valid and normalizes to E.164 format.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let normalized = normalize_phone_to_e164(&value.into())?;
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for PhoneNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for PhoneNumber {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for PhoneNumber {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PhoneNumber> for String {
    fn from(value: PhoneNumber) -> Self {
        value.0
    }
}

/// Six-digit one-time password wrapper.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OtpCode(String);

impl OtpCode {
    /// Ensures the code is exactly six ASCII digits.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let code = value.into();
        if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(code))
        } else {
            Err(TypeConstraintError::InvalidOtpCode)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for OtpCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for OtpCode {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for OtpCode {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<OtpCode> for String {
    fn from(value: OtpCode) -> Self {
        value.0
    }
}

/// Product quantity; must be positive.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProductQuantity(i32);

impl ProductQuantity {
    /// Construct a new quantity ensuring it is above zero.
    pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
        if value > 0 {
            Ok(Self(value))
        } else {
            Err(TypeConstraintError::InvalidValue(
                "quantity must be greater than zero".to_string(),
            ))
        }
    }

    /// Return the raw integer quantity.
    pub const fn get(self) -> i32 {
        self.0
    }
}

impl Display for ProductQuantity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i32> for ProductQuantity {
    type Error = TypeConstraintError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ProductQuantity> for i32 {
    fn from(value: ProductQuantity) -> Self {
        value.0
    }
}

non_empty_string_newtype!(OrderReference, "Optional order reference wrapper.");

non_empty_string_newtype!(OrderNotes, "Optional order notes wrapper.");

non_empty_string_newtype!(OrderShippingAddress, "Order shipping address wrapper.");
non_empty_string_newtype!(OrderConsignee, "Order consignee wrapper.");
non_empty_string_newtype!(OrderDeliveryNotes, "Order delivery notes wrapper.");
non_empty_string_newtype!(OrderPayer, "Order payer wrapper.");
