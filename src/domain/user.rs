use chrono::NaiveDateTime;
use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

use crate::domain::types::{HubId, TypeConstraintError, UserEmail, UserId, UserName};

/// Domain representation of a user belonging to a hub.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique identifier of the user.
    pub id: UserId,
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable display name.
    pub name: UserName,
    /// Primary email address expected to be normalised to lowercase by the caller.
    pub email: UserEmail,
}

/// Payload required to insert a new user for a hub.
#[derive(Clone, Debug, Deserialize)]
pub struct NewUser {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Human-readable display name.
    pub name: UserName,
    /// Primary email address expected to be normalised to lowercase by the caller.
    pub email: UserEmail,
}

impl NewUser {
    /// Build a new user payload from already validated value objects.
    #[must_use]
    pub fn new(hub_id: HubId, name: UserName, email: UserEmail) -> Self {
        Self {
            hub_id,
            name,
            email,
        }
    }

    /// Attempt to construct a new user payload from raw inputs by enforcing domain constraints.
    pub fn try_new(
        hub_id: i32,
        name: impl Into<String>,
        email: impl Into<String>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            UserName::new(name)?,
            UserEmail::new(email)?,
        ))
    }
}

/// Patch data applied when updating an existing user.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateUser {
    /// Updated human-readable display name.
    pub name: UserName,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl UpdateUser {
    /// Construct an update payload using pre-sanitised inputs from the caller.
    #[must_use]
    pub fn new(name: UserName) -> Self {
        let updated_at = chrono::Local::now().naive_utc();
        Self { name, updated_at }
    }

    /// Attempt to construct an update payload by validating the provided name.
    pub fn try_new(name: impl Into<String>) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(UserName::new(name)?))
    }
}

impl TryFrom<&AuthenticatedUser> for NewUser {
    type Error = TypeConstraintError;

    /// Create a new user payload from an authenticated user context.
    fn try_from(value: &AuthenticatedUser) -> Result<Self, Self::Error> {
        Self::try_new(value.hub_id, value.name.clone(), value.email.clone())
    }
}
