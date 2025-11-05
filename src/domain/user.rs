use chrono::NaiveDateTime;
use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

/// Domain representation of a user belonging to a hub.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct User {
    /// Unique identifier of the user.
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable display name.
    pub name: String,
    /// Primary email address expected to be normalised to lowercase by the caller.
    pub email: String,
}

/// Payload required to insert a new user for a hub.
#[derive(Clone, Debug, Deserialize)]
pub struct NewUser {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Human-readable display name.
    pub name: String,
    /// Primary email address expected to be normalised to lowercase by the caller.
    pub email: String,
}

impl NewUser {
    /// Build a new user payload from pre-sanitised inputs supplied by the caller.
    #[must_use]
    pub fn new(hub_id: i32, name: impl Into<String>, email: impl Into<String>) -> Self {
        let name = name.into();
        let email = email.into();
        Self {
            hub_id,
            name,
            email,
        }
    }
}

/// Patch data applied when updating an existing user.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateUser {
    /// Updated human-readable display name.
    pub name: String,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl UpdateUser {
    /// Construct an update payload using pre-sanitised inputs from the caller.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let updated_at = chrono::Local::now().naive_utc();
        Self { name, updated_at }
    }
}

impl From<&AuthenticatedUser> for NewUser {
    /// Create a new user payload from an authenticated user context.
    fn from(value: &AuthenticatedUser) -> Self {
        NewUser::new(value.hub_id, value.name.clone(), value.email.clone())
    }
}
