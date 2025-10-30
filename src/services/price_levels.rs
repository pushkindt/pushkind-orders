use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;
use serde::{Deserialize, Serialize};

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::customer::{CustomerListQuery, NewCustomer};
use crate::domain::price_level::{PriceLevel, PriceLevelListQuery};
use crate::forms::price_levels::{
    AddPriceLevelForm, AssignClientPriceLevelPayload, EditPriceLevelForm, UploadPriceLevelsForm,
};
use crate::repository::{CustomerReader, CustomerWriter, PriceLevelReader, PriceLevelWriter};
use crate::services::{ServiceError, ServiceResult};

/// Query parameters accepted by the price levels index page.
#[derive(Debug, Default, Deserialize)]
pub struct PriceLevelsQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
}

/// Data required to render the price levels index template.
pub struct PriceLevelsPageData {
    /// Paginated list of price levels to show in the table.
    pub price_levels: Vec<PriceLevel>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}

/// Saved price level assignment for a specific customer.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientPriceLevelAssignment {
    /// Normalized email address used to identify the customer.
    pub email: String,
    /// Optional phone number used to disambiguate customers with the same email.
    pub phone: Option<String>,
    /// Selected price level identifier, if any.
    pub price_level_id: Option<i32>,
}

/// Aggregated client assignments together with the hub default.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientPriceLevelAssignments {
    /// Owning hub identifier for the assignments.
    pub hub_id: i32,
    /// Default price level identifier configured for the hub.
    pub default_price_level_id: Option<i32>,
    /// Saved assignments for customers belonging to the hub.
    pub assignments: Vec<ClientPriceLevelAssignment>,
}

/// Loads the price levels list for the index page.
pub fn load_price_levels<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: PriceLevelsQuery,
) -> ServiceResult<PriceLevelsPageData>
where
    R: PriceLevelReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let mut list_query = PriceLevelListQuery::new(user.hub_id);

    if let Some(value) = query.search.as_ref() {
        list_query = list_query.search(value);
    }

    let (_total, price_levels) = repo
        .list_price_levels(list_query)
        .map_err(ServiceError::from)?;

    Ok(PriceLevelsPageData {
        price_levels,
        search: query.search,
    })
}

/// Loads saved price level assignments for all hub customers.
pub fn load_client_price_level_assignments<R>(
    repo: &R,
    user: &AuthenticatedUser,
) -> ServiceResult<ClientPriceLevelAssignments>
where
    R: PriceLevelReader + CustomerReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let (_, price_levels) = repo
        .list_price_levels(PriceLevelListQuery::new(user.hub_id))
        .map_err(ServiceError::from)?;

    let default_price_level_id = price_levels
        .iter()
        .find(|level| level.is_default)
        .map(|level| level.id);

    let (_, customers) = repo
        .list_customers(CustomerListQuery::new(user.hub_id))
        .map_err(ServiceError::from)?;

    let assignments = customers
        .into_iter()
        .map(|customer| ClientPriceLevelAssignment {
            email: customer.email,
            phone: customer.phone,
            price_level_id: customer.price_level_id,
        })
        .collect();

    Ok(ClientPriceLevelAssignments {
        hub_id: user.hub_id,
        default_price_level_id,
        assignments,
    })
}

/// Creates a new price level for the authenticated user's hub.
pub fn create_price_level<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddPriceLevelForm,
) -> ServiceResult<PriceLevel>
where
    R: PriceLevelWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let new_price_level = form
        .into_new_price_level(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_price_level(&new_price_level)
        .map_err(ServiceError::from)
}

/// Updates an existing price level for the authenticated user's hub.
pub fn update_price_level<R>(
    repo: &R,
    user: &AuthenticatedUser,
    price_level_id: i32,
    form: EditPriceLevelForm,
) -> ServiceResult<PriceLevel>
where
    R: PriceLevelWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let updates = form
        .into_update_price_level()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.update_price_level(price_level_id, user.hub_id, &updates)
        .map_err(ServiceError::from)
}

/// Imports price levels from an uploaded CSV file.
pub fn import_price_levels<R>(
    repo: &R,
    user: &AuthenticatedUser,
    mut form: UploadPriceLevelsForm,
) -> ServiceResult<usize>
where
    R: PriceLevelWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let price_levels = form
        .into_new_price_levels(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let count = price_levels.len();

    for level in &price_levels {
        repo.create_price_level(level).map_err(ServiceError::from)?;
    }

    Ok(count)
}

/// Deletes a price level for the authenticated user's hub.
pub fn remove_price_level<R>(
    repo: &R,
    user: &AuthenticatedUser,
    price_level_id: i32,
) -> ServiceResult<()>
where
    R: PriceLevelWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    repo.delete_price_level(price_level_id, user.hub_id)
        .map_err(ServiceError::from)
}

/// Persists a price level assignment for a single customer.
pub fn assign_price_level_to_client<R>(
    repo: &R,
    user: &AuthenticatedUser,
    payload: AssignClientPriceLevelPayload,
) -> ServiceResult<()>
where
    R: CustomerReader + CustomerWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let assignment = payload
        .into_assignment_request()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    if assignment.hub_id != user.hub_id {
        return Err(ServiceError::Unauthorized);
    }

    let customer = match repo
        .get_customer_by_email_and_phone(
            &assignment.email,
            assignment.phone.as_deref(),
            user.hub_id,
        )
        .map_err(ServiceError::from)?
    {
        Some(existing) => existing,
        None => {
            let mut new_customer = NewCustomer::new(
                assignment.hub_id,
                assignment.name.clone(),
                &assignment.email,
            );

            if let Some(phone) = assignment.phone.as_ref() {
                new_customer = new_customer.with_phone(phone.clone());
            }

            if let Some(price_level_id) = assignment.price_level_id {
                new_customer = new_customer.with_price_level_id(price_level_id);
            }

            repo.create_customer(&new_customer)
                .map_err(ServiceError::from)?
        }
    };

    repo.assign_price_level_to_customers(user.hub_id, &[customer.id], assignment.price_level_id)
        .map_err(ServiceError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::{Arc, Mutex};

    use actix_multipart::form::tempfile::TempFile;
    use tempfile::NamedTempFile;

    use crate::domain::customer::{Customer, CustomerListQuery, NewCustomer};
    use crate::domain::price_level::PriceLevel;
    use crate::forms::price_levels::{
        AddPriceLevelForm, AssignClientPriceLevelPayload, UploadPriceLevelsForm,
    };
    use crate::repository::mock::{
        MockCustomerReader, MockCustomerWriter, MockPriceLevelReader, MockPriceLevelWriter,
    };
    use crate::repository::{CustomerReader, CustomerWriter};
    use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

    struct CombinedCustomerRepo {
        reader: MockCustomerReader,
        writer: MockCustomerWriter,
    }

    impl CombinedCustomerRepo {
        fn new(reader: MockCustomerReader, writer: MockCustomerWriter) -> Self {
            Self { reader, writer }
        }
    }

    impl CustomerReader for CombinedCustomerRepo {
        fn get_customer_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Customer>> {
            self.reader.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_email(
            &self,
            email: &str,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>> {
            self.reader.get_customer_by_email(email, hub_id)
        }

        fn get_customer_by_email_and_phone(
            &self,
            email: &str,
            phone: Option<&str>,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>> {
            self.reader
                .get_customer_by_email_and_phone(email, phone, hub_id)
        }

        fn list_customers(
            &self,
            query: CustomerListQuery,
        ) -> RepositoryResult<(usize, Vec<Customer>)> {
            self.reader.list_customers(query)
        }
    }

    impl CustomerWriter for CombinedCustomerRepo {
        fn create_customer(&self, new_customer: &NewCustomer) -> RepositoryResult<Customer> {
            self.writer.create_customer(new_customer)
        }

        fn assign_price_level_to_customers(
            &self,
            hub_id: i32,
            customer_ids: &[i32],
            price_level_id: Option<i32>,
        ) -> RepositoryResult<()> {
            self.writer
                .assign_price_level_to_customers(hub_id, customer_ids, price_level_id)
        }
    }

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_level(id: i32, hub_id: i32, name: &str) -> PriceLevel {
        PriceLevel {
            id,
            hub_id,
            name: name.to_string(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            is_default: false,
        }
    }

    fn sample_customer(id: i32, hub_id: i32, price_level_id: Option<i32>) -> Customer {
        Customer {
            id,
            hub_id,
            name: format!("Customer {id}"),
            email: format!("customer{id}@example.com"),
            phone: Some(format!("+100000{id}")),
            price_level_id,
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 42,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    #[test]
    fn load_price_levels_returns_unauthorized_when_role_missing() {
        let repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[]);

        let result = load_price_levels(&repo, &user, PriceLevelsQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_price_levels_returns_paginated_data() {
        let mut repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = PriceLevelsQuery {
            search: Some("sil".to_string()),
        };

        let expected_hub = user.hub_id;

        repo.expect_list_price_levels()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id, expected_hub);
                assert_eq!(query.search.as_deref(), Some("sil"));
                true
            })
            .returning(move |_| {
                Ok((
                    5,
                    vec![
                        sample_level(1, expected_hub, "Silver"),
                        sample_level(2, expected_hub, "Gold"),
                    ],
                ))
            });

        let result = load_price_levels(&repo, &user, query);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.search.as_deref(), Some("sil"));
        assert_eq!(data.price_levels.len(), 2);
    }

    #[test]
    fn create_price_level_requires_role() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[]);
        let form = AddPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
        };

        let result = create_price_level(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn create_price_level_persists_price_level() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
        };

        let expected_hub = user.hub_id;
        repo.expect_create_price_level()
            .times(1)
            .withf(move |payload| payload.hub_id == expected_hub && payload.name == "Retail")
            .returning(move |_| Ok(sample_level(5, expected_hub, "Retail")));

        let result = create_price_level(&repo, &user, form).expect("expected success");

        assert_eq!(result.id, 5);
        assert_eq!(result.hub_id, expected_hub);
        assert_eq!(result.name, "Retail");
    }

    #[test]
    fn create_price_level_propagates_form_errors() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "   ".to_string(),
            default: false,
        };

        let result = create_price_level(&repo, &user, form);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(
                    message.contains("cannot be empty"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn update_price_level_requires_role() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[]);
        let form = EditPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
        };

        let result = update_price_level(&repo, &user, 7, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_price_level_updates_record() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditPriceLevelForm {
            name: "  Retail Plus  ".to_string(),
            default: true,
        };

        let expected_hub = user.hub_id;
        repo.expect_update_price_level()
            .times(1)
            .withf(move |id, hub, updates| {
                *id == 7
                    && *hub == expected_hub
                    && updates.name == "Retail Plus"
                    && updates.is_default
            })
            .return_once(move |_, _, _| Ok(sample_level(7, expected_hub, "Retail Plus")));

        let result = update_price_level(&repo, &user, 7, form).expect("expected success");

        assert_eq!(result.id, 7);
        assert_eq!(result.name, "Retail Plus");
    }

    #[test]
    fn update_price_level_propagates_form_errors() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditPriceLevelForm {
            name: "   ".to_string(),
            default: false,
        };

        let result = update_price_level(&repo, &user, 3, form);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(
                    message.contains("cannot be empty"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn update_price_level_bubbles_not_found() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
        };

        repo.expect_update_price_level()
            .times(1)
            .return_once(|_, _, _| Err(RepositoryError::NotFound));

        let result = update_price_level(&repo, &user, 11, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    struct ClientAssignmentRepo {
        customer_reader: MockCustomerReader,
        price_level_reader: MockPriceLevelReader,
    }

    impl ClientAssignmentRepo {
        fn new() -> Self {
            Self {
                customer_reader: MockCustomerReader::new(),
                price_level_reader: MockPriceLevelReader::new(),
            }
        }
    }

    impl CustomerReader for ClientAssignmentRepo {
        fn get_customer_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_email(
            &self,
            email: &str,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_email(email, hub_id)
        }

        fn get_customer_by_email_and_phone(
            &self,
            email: &str,
            phone: Option<&str>,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>> {
            self.customer_reader
                .get_customer_by_email_and_phone(email, phone, hub_id)
        }

        fn list_customers(
            &self,
            query: CustomerListQuery,
        ) -> RepositoryResult<(usize, Vec<Customer>)> {
            self.customer_reader.list_customers(query)
        }
    }

    impl PriceLevelReader for ClientAssignmentRepo {
        fn get_price_level_by_id(
            &self,
            id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Option<PriceLevel>> {
            self.price_level_reader.get_price_level_by_id(id, hub_id)
        }

        fn list_price_levels(
            &self,
            query: PriceLevelListQuery,
        ) -> RepositoryResult<(usize, Vec<PriceLevel>)> {
            self.price_level_reader.list_price_levels(query)
        }
    }

    #[test]
    fn load_client_price_level_assignments_requires_role() {
        let repo = ClientAssignmentRepo::new();
        let user = user_with_roles(&[]);

        let result = load_client_price_level_assignments(&repo, &user);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_client_price_level_assignments_returns_assignments() {
        let mut repo = ClientAssignmentRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;

        repo.price_level_reader
            .expect_list_price_levels()
            .withf(move |query| query.hub_id == hub_id)
            .returning(move |_| {
                Ok((
                    2,
                    vec![
                        PriceLevel {
                            is_default: true,
                            ..sample_level(10, hub_id, "Retail")
                        },
                        sample_level(11, hub_id, "Wholesale"),
                    ],
                ))
            });

        repo.customer_reader
            .expect_list_customers()
            .withf(move |query| query.hub_id == hub_id)
            .returning(move |_| {
                Ok((
                    2,
                    vec![
                        sample_customer(1, hub_id, Some(11)),
                        sample_customer(2, hub_id, None),
                    ],
                ))
            });

        let assignments =
            load_client_price_level_assignments(&repo, &user).expect("expected success");

        assert_eq!(assignments.hub_id, hub_id);
        assert_eq!(assignments.default_price_level_id, Some(10));
        assert_eq!(assignments.assignments.len(), 2);
        assert_eq!(
            assignments.assignments[0],
            ClientPriceLevelAssignment {
                email: "customer1@example.com".to_string(),
                phone: Some("+1000001".to_string()),
                price_level_id: Some(11),
            }
        );
        assert_eq!(
            assignments.assignments[1],
            ClientPriceLevelAssignment {
                email: "customer2@example.com".to_string(),
                phone: Some("+1000002".to_string()),
                price_level_id: None,
            }
        );
    }

    #[test]
    fn assign_price_level_to_client_requires_role() {
        let repo = CombinedCustomerRepo::new(MockCustomerReader::new(), MockCustomerWriter::new());
        let user = user_with_roles(&[]);
        let payload = AssignClientPriceLevelPayload {
            hub_id: user.hub_id,
            name: "Client Example".to_string(),
            email: "example@client.com".to_string(),
            phone: Some("+1234567890".to_string()),
            price_level_id: Some(5),
        };

        let result = assign_price_level_to_client(&repo, &user, payload);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn assign_price_level_to_client_updates_assignment_using_contact_lookup() {
        let mut reader = MockCustomerReader::new();
        let mut writer = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;
        let expected_customer_id = 321;

        reader
            .expect_get_customer_by_email_and_phone()
            .times(1)
            .withf(move |email, phone, query_hub_id| {
                *query_hub_id == hub_id
                    && email == "customer7@example.com"
                    && phone
                        .as_ref()
                        .map(|value| *value == "+15550007")
                        .unwrap_or(false)
            })
            .returning(move |_, _, _| {
                Ok(Some(sample_customer(expected_customer_id, hub_id, None)))
            });

        writer
            .expect_assign_price_level_to_customers()
            .times(1)
            .withf(move |target_hub, ids, price_level_id| {
                *target_hub == hub_id && ids == [expected_customer_id] && price_level_id == &Some(8)
            })
            .returning(|_, _, _| Ok(()));

        let repo = CombinedCustomerRepo::new(reader, writer);
        let payload = AssignClientPriceLevelPayload {
            hub_id,
            name: "Customer Seven".to_string(),
            email: "Customer7@Example.com ".to_string(),
            phone: Some("  +15550007 ".to_string()),
            price_level_id: Some(8),
        };

        assign_price_level_to_client(&repo, &user, payload).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_clears_assignment_without_phone() {
        let mut reader = MockCustomerReader::new();
        let mut writer = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;
        let expected_customer = Customer {
            id: 55,
            hub_id,
            name: "Client 55".to_string(),
            email: "client55@example.com".to_string(),
            phone: None,
            price_level_id: Some(12),
        };
        let expected_customer_id = expected_customer.id;

        reader
            .expect_get_customer_by_email_and_phone()
            .times(1)
            .withf(move |email, phone, query_hub_id| {
                *query_hub_id == hub_id && email == "client55@example.com" && phone.is_none()
            })
            .returning(move |_, _, _| Ok(Some(expected_customer.clone())));

        writer
            .expect_assign_price_level_to_customers()
            .times(1)
            .withf(move |target_hub, ids, price_level_id| {
                *target_hub == hub_id && ids == [expected_customer_id] && price_level_id.is_none()
            })
            .returning(|_, _, _| Ok(()));

        let repo = CombinedCustomerRepo::new(reader, writer);
        let payload = AssignClientPriceLevelPayload {
            hub_id,
            name: "Client 55".to_string(),
            email: "client55@example.com".to_string(),
            phone: None,
            price_level_id: None,
        };

        assign_price_level_to_client(&repo, &user, payload).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_creates_customer_when_lookup_missing() {
        let mut reader = MockCustomerReader::new();
        let mut writer = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;
        let expected_customer_id = 777;

        reader
            .expect_get_customer_by_email_and_phone()
            .times(1)
            .returning(|_, _, _| Ok(None));

        writer
            .expect_create_customer()
            .times(1)
            .withf(move |new_customer| {
                new_customer.hub_id == hub_id
                    && new_customer.email == "missing@example.com"
                    && new_customer.name == "Missing User"
                    && new_customer
                        .phone
                        .as_ref()
                        .map(|value| value == "+1999000")
                        .unwrap_or(false)
                    && new_customer.price_level_id == Some(1)
            })
            .returning(move |new_customer| {
                Ok(Customer {
                    id: expected_customer_id,
                    hub_id,
                    name: new_customer.name.clone(),
                    email: new_customer.email.clone(),
                    phone: new_customer.phone.clone(),
                    price_level_id: new_customer.price_level_id,
                })
            });

        writer
            .expect_assign_price_level_to_customers()
            .times(1)
            .withf(move |target_hub, ids, price_level_id| {
                *target_hub == hub_id && ids == [expected_customer_id] && price_level_id == &Some(1)
            })
            .return_once(|_, _, _| Ok(()));

        let repo = CombinedCustomerRepo::new(reader, writer);
        let payload = AssignClientPriceLevelPayload {
            hub_id,
            name: "  Missing User  ".to_string(),
            email: " Missing@Example.com ".to_string(),
            phone: Some(" +1999000 ".to_string()),
            price_level_id: Some(1),
        };

        assign_price_level_to_client(&repo, &user, payload).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_propagates_form_errors() {
        let repo = CombinedCustomerRepo::new(MockCustomerReader::new(), MockCustomerWriter::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let payload = AssignClientPriceLevelPayload {
            hub_id: user.hub_id,
            name: "".to_string(),
            email: "".to_string(),
            phone: None,
            price_level_id: Some(0),
        };

        let result = assign_price_level_to_client(&repo, &user, payload);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(message.contains("invalid_price_level_id"));
                assert!(message.contains("empty_email"));
                assert!(message.contains("empty_name"));
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn import_price_levels_requires_role() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[]);
        let form = build_upload_form("name\nRetail\n");

        let result = import_price_levels(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn import_price_levels_creates_all_levels() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = build_upload_form("name\nRetail\nWholesale\n");

        let captured_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let names_clone = Arc::clone(&captured_names);

        repo.expect_create_price_level()
            .times(2)
            .returning(move |payload| {
                let mut guard = names_clone.lock().expect("mutex poisoned");
                guard.push(payload.name.clone());
                Ok(sample_level(
                    guard.len() as i32,
                    payload.hub_id,
                    &payload.name,
                ))
            });

        let result = import_price_levels(&repo, &user, form).expect("expected success");

        assert_eq!(result, 2);

        let stored = captured_names.lock().expect("mutex poisoned");
        assert_eq!(stored.len(), 2);
        assert!(stored.contains(&"Retail".to_string()));
        assert!(stored.contains(&"Wholesale".to_string()));
    }

    #[test]
    fn import_price_levels_handles_empty_upload() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = build_upload_form("name\n");

        let result = import_price_levels(&repo, &user, form).expect("expected success");

        assert_eq!(result, 0);
    }

    #[test]
    fn remove_price_level_requires_role() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[]);

        let result = remove_price_level(&repo, &user, 42);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn remove_price_level_bubbles_not_found() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_price_level()
            .times(1)
            .withf(|id, hub| *id == 99 && *hub == 42)
            .return_once(|_, _| Err(RepositoryError::NotFound));

        let result = remove_price_level(&repo, &user, 99);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn remove_price_level_succeeds() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_price_level()
            .times(1)
            .withf(|id, hub| *id == 7 && *hub == 42)
            .return_once(|_, _| Ok(()));

        remove_price_level(&repo, &user, 7).expect("expected success");
    }

    fn build_upload_form(csv: &str) -> UploadPriceLevelsForm {
        let mut file = NamedTempFile::new().expect("create temp file");
        file.write_all(csv.as_bytes()).expect("write csv file");
        file.as_file_mut()
            .seek(SeekFrom::Start(0))
            .expect("seek to start");

        UploadPriceLevelsForm {
            csv: TempFile {
                file,
                content_type: None,
                file_name: Some("levels.csv".to_string()),
                size: csv.len(),
            },
        }
    }
}
