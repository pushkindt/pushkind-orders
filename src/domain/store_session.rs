//! Shared storefront session contract used by the Orders service.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

/// Cookie name used for storefront authentication across browser-facing services.
pub const STORE_SESSION_COOKIE_NAME: &str = "store-session";
/// Default storefront session lifetime in days.
pub const STORE_SESSION_TTL_DAYS: i64 = 7;

/// JWT claims representing an authenticated storefront client.
///
/// This contract is intentionally local to Orders for the first migration phase.
/// CRM is expected to issue a token with the same shape in its own local type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreSessionClaims {
    /// CRM client public identifier encoded as a UUID string.
    pub sub: String,
    /// Hub identifier that owns the storefront client.
    pub hub_id: i32,
    /// Client display name returned to the storefront UI.
    pub name: String,
    /// Client phone number in normalized E.164 format.
    pub phone: String,
    /// Optional email address mirrored from CRM.
    pub email: Option<String>,
    /// Expiration timestamp in seconds since the Unix epoch.
    pub exp: usize,
}

impl StoreSessionClaims {
    /// Update the expiration timestamp relative to the current time.
    pub fn set_expiration(&mut self, days: i64) {
        self.exp = match Utc::now().checked_add_signed(Duration::days(days)) {
            Some(expiration) => expiration.timestamp() as usize,
            None => self.exp,
        };
    }

    /// Returns `true` when the claims belong to the provided hub.
    #[must_use]
    pub fn matches_hub(&self, hub_id: i32) -> bool {
        self.hub_id == hub_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_claims() -> StoreSessionClaims {
        StoreSessionClaims {
            sub: "67e55044-10b1-426f-9247-bb680e5fe0c8".to_string(),
            hub_id: 7,
            name: "Alice".to_string(),
            phone: "+15551234567".to_string(),
            email: Some("alice@example.com".to_string()),
            exp: 0,
        }
    }

    #[test]
    fn set_expiration_updates_timestamp() {
        let mut claims = sample_claims();

        claims.set_expiration(STORE_SESSION_TTL_DAYS);

        assert!(claims.exp > 0);
    }

    #[test]
    fn matches_hub_checks_hub_id() {
        let claims = sample_claims();

        assert!(claims.matches_hub(7));
        assert!(!claims.matches_hub(8));
    }
}
