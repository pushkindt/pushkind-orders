use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};

use crate::domain::types::{HubId, VendorId};
use crate::domain::user::NewUser;
use crate::domain::vendor::{NewVendor, UpdateVendor, VendorListQuery};
use crate::dto::vendors::{VendorQuery, VendorUserView, VendorsPageData};
use crate::forms::vendors::{
    AddUserForm, AddUserPayload, AddVendorForm, AddVendorPayload, AssignVendorUserForm,
    AssignVendorUserPayload, ClearVendorUserForm, ClearVendorUserPayload, EditVendorForm,
    EditVendorPayload,
};
use crate::repository::{
    UserListQuery, UserReader, UserWriter, VendorReader, VendorUserReader, VendorUserWriter,
    VendorWriter,
};
use crate::services::{ServiceError, ServiceResult, ensure_admin};

/// Loads the vendors management page.
pub fn load_vendors_page<R>(
    query: VendorQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<VendorsPageData>
where
    R: VendorReader + VendorUserReader + UserReader + ?Sized,
{
    ensure_admin(user)?;

    let VendorQuery { search, page } = query;
    let page = page.unwrap_or(1);

    let hub_id = HubId::new(user.hub_id)?;
    let mut list_query = VendorListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(term) = search.as_ref() {
        list_query = list_query.search(term);
    }

    let (total, vendors) = repo.list_vendors(list_query)?;
    let vendor_choices = repo.list_vendors(VendorListQuery::new(hub_id))?.1;

    let vendor_lookup: HashMap<VendorId, String> = vendor_choices
        .iter()
        .map(|vendor| (vendor.id, vendor.name.as_str().to_string()))
        .collect();

    let (_, users) = repo.list_users(UserListQuery::new(hub_id))?;
    let mut user_views = Vec::new();
    for user in users {
        let vendor_id = repo.get_vendor_for_user(user.id, hub_id)?;
        let vendor_name = vendor_id.and_then(|id| vendor_lookup.get(&id).cloned());
        user_views.push(VendorUserView::from_user(
            user,
            vendor_id.map(|id| id.get()),
            vendor_name,
        ));
    }
    user_views.sort_by(|a, b| a.email.cmp(&b.email));

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let vendors = Paginated::new(vendors, page, total_pages);

    Ok(VendorsPageData {
        vendors,
        vendor_choices,
        users: user_views,
        search,
    })
}

/// Ensures the current user can access the vendors page shell.
pub fn ensure_vendors_page_access(user: &AuthenticatedUser) -> ServiceResult<()> {
    ensure_admin(user)
}

/// Creates a new user record.
pub fn add_user_from_payload<R>(
    payload: AddUserPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::user::User>
where
    R: UserWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let new_user = NewUser::new(hub_id, payload.name, payload.email);

    Ok(repo.create_user(&new_user)?)
}

/// Creates a new vendor for the authenticated user's hub.
pub fn create_vendor_from_payload<R>(
    payload: AddVendorPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::vendor::Vendor>
where
    R: VendorWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let new_vendor = NewVendor::new(hub_id, payload.name);

    Ok(repo.create_vendor(&new_vendor)?)
}

/// Loads a single vendor for editing.
pub fn load_vendor_for_edit<R>(
    vendor_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::vendor::Vendor>
where
    R: VendorReader + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let vendor_id = VendorId::new(vendor_id)?;

    match repo.get_vendor_by_id(vendor_id, hub_id)? {
        Some(vendor) => Ok(vendor),
        None => Err(ServiceError::NotFound),
    }
}

/// Updates an existing vendor for the authenticated user's hub.
pub fn modify_vendor_from_payload<R>(
    vendor_id: i32,
    payload: EditVendorPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::vendor::Vendor>
where
    R: VendorWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let path_vendor_id = VendorId::new(vendor_id)?;

    if path_vendor_id != payload.vendor_id {
        return Err(ServiceError::TypeConstraint(
            "Идентификатор поставщика указан неверно.".to_string(),
        ));
    }

    let updates = UpdateVendor::new(payload.name);

    Ok(repo.update_vendor(path_vendor_id, hub_id, &updates)?)
}

/// Deletes a vendor for the authenticated user's hub.
pub fn remove_vendor<R>(vendor_id: i32, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: VendorWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let vendor_id = VendorId::new(vendor_id)?;

    Ok(repo.delete_vendor(vendor_id, hub_id)?)
}

/// Assigns a user to a vendor in the authenticated user's hub.
pub fn assign_user_to_vendor_from_payload<R>(
    payload: AssignVendorUserPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: VendorUserWriter + VendorUserReader + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    if let Some(existing) = repo.get_vendor_for_user(payload.user_id, hub_id)? {
        if existing != payload.vendor_id {
            return Err(ServiceError::Conflict);
        }
        return Ok(());
    }

    Ok(repo.assign_user_to_vendor(payload.user_id, payload.vendor_id, hub_id)?)
}

/// Clears the vendor assignment for a user in the authenticated user's hub.
pub fn clear_vendor_for_user_from_payload<R>(
    payload: ClearVendorUserPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: VendorUserWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    Ok(repo.clear_vendor_for_user(payload.user_id, hub_id)?)
}

pub fn add_user<R>(form: AddUserForm, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: UserWriter + ?Sized,
{
    let payload: AddUserPayload = form.try_into()?;
    add_user_from_payload(payload, user, repo).map(|_| ())
}

pub fn create_vendor<R>(
    form: AddVendorForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::vendor::Vendor>
where
    R: VendorWriter + ?Sized,
{
    let payload: AddVendorPayload = form.try_into()?;
    create_vendor_from_payload(payload, user, repo)
}

pub fn modify_vendor<R>(
    form: EditVendorForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::vendor::Vendor>
where
    R: VendorWriter + ?Sized,
{
    let payload: EditVendorPayload = form.try_into()?;
    modify_vendor_from_payload(payload.vendor_id.get(), payload, user, repo)
}

pub fn assign_user_to_vendor<R>(
    form: AssignVendorUserForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: VendorUserWriter + VendorUserReader + ?Sized,
{
    let payload: AssignVendorUserPayload = form.try_into()?;
    assign_user_to_vendor_from_payload(payload, user, repo)
}

pub fn clear_vendor_for_user<R>(
    form: ClearVendorUserForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: VendorUserWriter + ?Sized,
{
    let payload: ClearVendorUserPayload = form.try_into()?;
    clear_vendor_for_user_from_payload(payload, user, repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ADMIN_ACCESS_ROLE;
    use crate::domain::types::{UserEmail, UserId};
    use crate::domain::user::User;
    use crate::repository::mock::{
        MockUserReader, MockVendorReader, MockVendorUserReader, MockVendorUserWriter,
        MockVendorWriter,
    };
    use pushkind_common::repository::errors::RepositoryResult;

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 7,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    #[test]
    fn load_vendors_page_requires_admin_role() {
        let repo = VendorServiceRepo::new();
        let user = user_with_roles(&[]);

        let result = load_vendors_page(VendorQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn assign_user_to_vendor_rejects_conflict() {
        let mut repo = VendorServiceRepo::new();
        let user = user_with_roles(&[ADMIN_ACCESS_ROLE]);
        let expected_hub = HubId::new(user.hub_id).unwrap();
        let user_id = UserId::new(5).unwrap();
        let vendor_id = VendorId::new(3).unwrap();

        repo.vendor_user_reader
            .expect_get_vendor_for_user()
            .times(1)
            .withf(move |id, hub| *id == user_id && *hub == expected_hub)
            .returning(|_, _| Ok(Some(VendorId::new(99).unwrap())));

        let form = AssignVendorUserForm {
            user_id: user_id.get(),
            vendor_id: vendor_id.get(),
        };

        let result = assign_user_to_vendor(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Conflict)));
    }

    struct VendorServiceRepo {
        vendor_reader: MockVendorReader,
        vendor_writer: MockVendorWriter,
        vendor_user_reader: MockVendorUserReader,
        vendor_user_writer: MockVendorUserWriter,
        user_reader: MockUserReader,
    }

    impl VendorServiceRepo {
        fn new() -> Self {
            Self {
                vendor_reader: MockVendorReader::new(),
                vendor_writer: MockVendorWriter::new(),
                vendor_user_reader: MockVendorUserReader::new(),
                vendor_user_writer: MockVendorUserWriter::new(),
                user_reader: MockUserReader::new(),
            }
        }
    }

    impl VendorReader for VendorServiceRepo {
        fn get_vendor_by_id(
            &self,
            vendor_id: VendorId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<crate::domain::vendor::Vendor>> {
            self.vendor_reader.get_vendor_by_id(vendor_id, hub_id)
        }

        fn list_vendors(
            &self,
            query: VendorListQuery,
        ) -> RepositoryResult<(usize, Vec<crate::domain::vendor::Vendor>)> {
            self.vendor_reader.list_vendors(query)
        }
    }

    impl VendorWriter for VendorServiceRepo {
        fn create_vendor(
            &self,
            new_vendor: &NewVendor,
        ) -> RepositoryResult<crate::domain::vendor::Vendor> {
            self.vendor_writer.create_vendor(new_vendor)
        }

        fn update_vendor(
            &self,
            vendor_id: VendorId,
            hub_id: HubId,
            updates: &UpdateVendor,
        ) -> RepositoryResult<crate::domain::vendor::Vendor> {
            self.vendor_writer.update_vendor(vendor_id, hub_id, updates)
        }

        fn delete_vendor(&self, vendor_id: VendorId, hub_id: HubId) -> RepositoryResult<()> {
            self.vendor_writer.delete_vendor(vendor_id, hub_id)
        }
    }

    impl VendorUserReader for VendorServiceRepo {
        fn get_vendor_for_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<VendorId>> {
            self.vendor_user_reader.get_vendor_for_user(user_id, hub_id)
        }
    }

    impl VendorUserWriter for VendorServiceRepo {
        fn assign_user_to_vendor(
            &self,
            user_id: UserId,
            vendor_id: VendorId,
            hub_id: HubId,
        ) -> RepositoryResult<()> {
            self.vendor_user_writer
                .assign_user_to_vendor(user_id, vendor_id, hub_id)
        }

        fn clear_vendor_for_user(&self, user_id: UserId, hub_id: HubId) -> RepositoryResult<()> {
            self.vendor_user_writer
                .clear_vendor_for_user(user_id, hub_id)
        }
    }

    impl UserReader for VendorServiceRepo {
        fn get_user_by_id(&self, id: UserId, hub_id: HubId) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &UserEmail,
            hub_id: HubId,
        ) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_email(email, hub_id)
        }

        fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)> {
            self.user_reader.list_users(query)
        }
    }
}
