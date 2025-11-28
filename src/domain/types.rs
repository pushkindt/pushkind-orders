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
}

/// Macro to generate lightweight newtypes for positive identifiers.
macro_rules! id_newtype {
    ($name:ident) => {
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

id_newtype!(UserId);
id_newtype!(HubId);
id_newtype!(TagId);
id_newtype!(ProductTagId);
id_newtype!(ProductId);
id_newtype!(CustomerId);
id_newtype!(CategoryId);
id_newtype!(ProductPriceLevelRateId);
id_newtype!(PriceLevelId);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Price stored in the smallest currency unit; must be positive.
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Lower-cased and validated email address.
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Wrapper for non-empty, trimmed strings.
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Optional user name wrapper enforcing non-empty values.
pub struct UserName(String);

impl UserName {
    /// Constructs a user name that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let name = NonEmptyString::new(value)?;
        Ok(Self(name.into_inner()))
    }

    /// Borrow the user name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the owned user name.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for UserName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for UserName {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for UserName {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UserName> for String {
    fn from(value: UserName) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Tag name wrapper enforcing non-empty values.
pub struct TagName(String);

impl TagName {
    /// Constructs a tag name that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let name = NonEmptyString::new(value)?;
        Ok(Self(name.into_inner()))
    }

    /// Borrow the tag name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the owned tag name.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for TagName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for TagName {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for TagName {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TagName> for String {
    fn from(value: TagName) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Customer name wrapper enforcing non-empty values.
pub struct CustomerName(String);

impl CustomerName {
    /// Constructs a customer name that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let name = NonEmptyString::new(value)?;
        Ok(Self(name.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Category name wrapper enforcing non-empty values.
pub struct CategoryName(String);

impl CategoryName {
    /// Constructs a category name that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let name = NonEmptyString::new(value)?;
        Ok(Self(name.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for CategoryName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CategoryName {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for CategoryName {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CategoryName> for String {
    fn from(value: CategoryName) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Product name wrapper enforcing non-empty values.
pub struct ProductName(String);

impl ProductName {
    /// Constructs a product name that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let name = NonEmptyString::new(value)?;
        Ok(Self(name.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for ProductName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ProductName {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ProductName {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ProductName> for String {
    fn from(value: ProductName) -> Self {
        value.0
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Price level name wrapper enforcing non-empty values.
pub struct PriceLevelName(String);

impl PriceLevelName {
    /// Constructs a price level name that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let name = NonEmptyString::new(value)?;
        Ok(Self(name.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for PriceLevelName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for PriceLevelName {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for PriceLevelName {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PriceLevelName> for String {
    fn from(value: PriceLevelName) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Optional SKU wrapper enforcing non-empty values.
pub struct ProductSku(String);

impl ProductSku {
    /// Constructs a SKU that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let sku = NonEmptyString::new(value)?;
        Ok(Self(sku.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for ProductSku {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ProductSku {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ProductSku {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ProductSku> for String {
    fn from(value: ProductSku) -> Self {
        value.0
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Units wrapper enforcing non-empty values.
pub struct ProductUnits(String);

impl ProductUnits {
    /// Constructs a unit string that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let units = NonEmptyString::new(value)?;
        Ok(Self(units.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for ProductUnits {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ProductUnits {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ProductUnits {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ProductUnits> for String {
    fn from(value: ProductUnits) -> Self {
        value.0
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// ISO 4217 currency code wrapper enforcing three uppercase ASCII letters.
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Optional descriptive text for categories.
pub struct CategoryDescription(String);

impl CategoryDescription {
    /// Constructs a description that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let description = NonEmptyString::new(value)?;
        Ok(Self(description.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for CategoryDescription {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for CategoryDescription {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Optional descriptive text for products.
pub struct ProductDescription(String);

impl ProductDescription {
    /// Constructs a description that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let description = NonEmptyString::new(value)?;
        Ok(Self(description.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for ProductDescription {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ProductDescription {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
/// Product amount; must be positive.
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Validated image URL wrapper.
pub struct ImageUrl(String);

impl ImageUrl {
    /// Constructs an image URL that is trimmed and non-empty.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let url = NonEmptyString::new(value)?;
        Ok(Self(url.into_inner()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for ImageUrl {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ImageUrl {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Display for CustomerName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CustomerName {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for CustomerName {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CustomerName> for String {
    fn from(value: CustomerName) -> Self {
        value.0
    }
}

/// Normalizes a phone number string to E.164 format.
pub fn normalize_phone_to_e164(value: &str) -> Result<String, TypeConstraintError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TypeConstraintError::EmptyString);
    }
    let parsed = parse(None, trimmed).map_err(|_| TypeConstraintError::InvalidPhone)?;
    Ok(parsed.format().mode(Mode::E164).to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Normalized phone number wrapper (expected E.164).
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Six-digit one-time password wrapper.
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
