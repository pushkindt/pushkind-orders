//! Service layer orchestrating domain logic and repository operations.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::ensure_role;

use crate::domain::types::{HubId, UserEmail, VendorId};
use crate::repository::{UserReader, VendorUserReader};
use crate::{ADMIN_ACCESS_ROLE, SERVICE_ACCESS_ROLE, VENDOR_ACCESS_ROLE};

pub mod api;
pub mod categories;
pub mod main;
pub mod orders;
pub mod price_levels;
pub mod products;
pub mod store;
pub mod tags;
pub mod vendors;

pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

/// Access scope resolved for hub users.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubAccessScope {
    /// Basic access without special roles.
    Basic,
    /// Full hub access for operators.
    Admin,
    /// Vendor-scoped access for vendor users.
    Vendor { vendor_id: VendorId },
}

fn has_role(user: &AuthenticatedUser, role: &str) -> bool {
    user.roles.iter().any(|item| item == role)
}

/// Enforce full hub operator access.
pub fn ensure_admin(user: &AuthenticatedUser) -> ServiceResult<()> {
    ensure_role(user, ADMIN_ACCESS_ROLE)
}

/// Return true when the user can mutate catalog data.
pub fn has_catalog_write_access(user: &AuthenticatedUser) -> bool {
    has_role(user, ADMIN_ACCESS_ROLE) || has_role(user, VENDOR_ACCESS_ROLE)
}

/// Enforce write access to hub catalog pages.
pub fn ensure_catalog_write_access(user: &AuthenticatedUser) -> ServiceResult<()> {
    if has_catalog_write_access(user) {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}

/// Enforce read-only access to hub configuration pages.
pub fn ensure_catalog_read_access(user: &AuthenticatedUser) -> ServiceResult<()> {
    if has_role(user, SERVICE_ACCESS_ROLE)
        || has_role(user, ADMIN_ACCESS_ROLE)
        || has_role(user, VENDOR_ACCESS_ROLE)
    {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}

/// Resolve the hub access scope for an authenticated user.
pub fn resolve_hub_access<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<HubAccessScope>
where
    R: UserReader + VendorUserReader + ?Sized,
{
    if !has_role(user, SERVICE_ACCESS_ROLE) {
        return Err(ServiceError::Unauthorized);
    }
    if has_role(user, ADMIN_ACCESS_ROLE) {
        Ok(HubAccessScope::Admin)
    } else if has_role(user, VENDOR_ACCESS_ROLE) {
        let hub_id = HubId::new(user.hub_id)?;
        let email = UserEmail::new(user.email.clone())?;
        let user_record = repo
            .get_user_by_email(&email, hub_id)?
            .ok_or(ServiceError::Unauthorized)?;

        let vendor_id = repo
            .get_vendor_for_user(user_record.id, hub_id)?
            .ok_or(ServiceError::Unauthorized)?;

        Ok(HubAccessScope::Vendor { vendor_id })
    } else {
        Ok(HubAccessScope::Basic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{HubId, UserEmail, UserId, UserName, VendorId};
    use crate::domain::user::User;
    use crate::repository::mock::{MockUserReader, MockVendorUserReader};
    use pushkind_common::repository::errors::RepositoryResult;

    struct FakeRepo {
        user_reader: MockUserReader,
        vendor_user_reader: MockVendorUserReader,
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                user_reader: MockUserReader::new(),
                vendor_user_reader: MockVendorUserReader::new(),
            }
        }
    }

    impl UserReader for FakeRepo {
        fn get_user_by_id(&self, user_id: UserId, hub_id: HubId) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(user_id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &UserEmail,
            hub_id: HubId,
        ) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_email(email, hub_id)
        }

        fn list_users(
            &self,
            query: crate::repository::UserListQuery,
        ) -> RepositoryResult<(usize, Vec<User>)> {
            self.user_reader.list_users(query)
        }
    }

    impl VendorUserReader for FakeRepo {
        fn get_vendor_for_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<VendorId>> {
            self.vendor_user_reader.get_vendor_for_user(user_id, hub_id)
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 11,
            name: "User".to_string(),
            roles: roles.iter().map(|role| role.to_string()).collect(),
            exp: 0,
        }
    }

    #[test]
    fn ensure_catalog_read_access_allows_admin_vendor_and_service() {
        let admin = user_with_roles(&[ADMIN_ACCESS_ROLE]);
        let vendor = user_with_roles(&[VENDOR_ACCESS_ROLE]);
        let service = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        assert!(ensure_catalog_read_access(&admin).is_ok());
        assert!(ensure_catalog_read_access(&vendor).is_ok());
        assert!(ensure_catalog_read_access(&service).is_ok());
    }

    #[test]
    fn ensure_catalog_read_access_rejects_unknown_role() {
        let user = user_with_roles(&["other"]);
        assert!(matches!(
            ensure_catalog_read_access(&user),
            Err(ServiceError::Unauthorized)
        ));
    }

    #[test]
    fn resolve_hub_access_returns_admin_for_admin_role() {
        let repo = FakeRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE, ADMIN_ACCESS_ROLE]);

        let access = resolve_hub_access(&user, &repo).expect("access");
        assert!(matches!(access, HubAccessScope::Admin));
    }

    #[test]
    fn resolve_hub_access_returns_basic_for_service_role() {
        let repo = FakeRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let access = resolve_hub_access(&user, &repo).expect("access");
        assert!(matches!(access, HubAccessScope::Basic));
    }

    #[test]
    fn resolve_hub_access_returns_vendor_scope_for_vendor_role() {
        let mut repo = FakeRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE, VENDOR_ACCESS_ROLE]);
        let hub_id = HubId::new(user.hub_id).unwrap();
        let user_id = UserId::new(12).unwrap();
        let vendor_id = VendorId::new(5).unwrap();
        let user_record = User {
            id: user_id,
            hub_id,
            name: UserName::new("Vendor User").unwrap(),
            email: UserEmail::new(user.email.clone()).unwrap(),
        };

        repo.user_reader
            .expect_get_user_by_email()
            .times(1)
            .withf(|email, hub| email.as_str() == "user@example.com" && hub.get() == 11)
            .returning(move |_, _| Ok(Some(user_record.clone())));

        repo.vendor_user_reader
            .expect_get_vendor_for_user()
            .times(1)
            .withf(move |id, hub| *id == user_id && hub.get() == 11)
            .returning(move |_, _| Ok(Some(vendor_id)));

        let access = resolve_hub_access(&user, &repo).expect("access");
        assert!(matches!(access, HubAccessScope::Vendor { vendor_id: id } if id == vendor_id));
    }

    #[test]
    fn resolve_hub_access_rejects_missing_vendor_assignment() {
        let mut repo = FakeRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE, VENDOR_ACCESS_ROLE]);
        let hub_id = HubId::new(user.hub_id).unwrap();
        let user_id = UserId::new(44).unwrap();
        let user_record = User {
            id: user_id,
            hub_id,
            name: UserName::new("Vendor User").unwrap(),
            email: UserEmail::new(user.email.clone()).unwrap(),
        };

        repo.user_reader
            .expect_get_user_by_email()
            .times(1)
            .returning(move |_, _| Ok(Some(user_record.clone())));

        repo.vendor_user_reader
            .expect_get_vendor_for_user()
            .times(1)
            .returning(|_, _| Ok(None));

        let access = resolve_hub_access(&user, &repo);
        assert!(matches!(access, Err(ServiceError::Unauthorized)));
    }
}
