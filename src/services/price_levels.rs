use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::{Deserialize, Serialize};

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::customer::CustomerListQuery;
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
    /// Page number requested by the user interface.
    pub page: Option<usize>,
}

/// Data required to render the price levels index template.
pub struct PriceLevelsPageData {
    /// Paginated list of price levels to show in the table.
    pub price_levels: Paginated<PriceLevel>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}

/// Saved price level assignment for a specific customer.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientPriceLevelAssignment {
    /// Identifier of the customer that owns the assignment.
    pub customer_id: i32,
    /// Selected price level identifier, if any.
    pub price_level_id: Option<i32>,
}

/// Aggregated client assignments together with the hub default.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientPriceLevelAssignments {
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

    let page = query.page.unwrap_or(1);
    let mut list_query = PriceLevelListQuery::new(user.hub_id);

    if let Some(value) = query.search.as_ref() {
        list_query = list_query.search(value);
    }

    list_query = list_query.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let (total, price_levels) = repo
        .list_price_levels(list_query)
        .map_err(ServiceError::from)?;

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let price_levels = Paginated::new(price_levels, page, total_pages);

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
            customer_id: customer.id,
            price_level_id: customer.price_level_id,
        })
        .collect();

    Ok(ClientPriceLevelAssignments {
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
    customer_id: i32,
    payload: AssignClientPriceLevelPayload,
) -> ServiceResult<()>
where
    R: CustomerWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let price_level_id = payload
        .into_price_level_id()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.assign_price_level_to_customers(user.hub_id, &[customer_id], price_level_id)
        .map_err(ServiceError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::Value;
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::{Arc, Mutex};

    use actix_multipart::form::tempfile::TempFile;
    use tempfile::NamedTempFile;

    use crate::domain::customer::Customer;
    use crate::domain::price_level::PriceLevel;
    use crate::forms::price_levels::{
        AddPriceLevelForm, AssignClientPriceLevelPayload, UploadPriceLevelsForm,
    };
    use crate::repository::mock::{
        MockCustomerReader, MockCustomerWriter, MockPriceLevelReader, MockPriceLevelWriter,
    };
    use pushkind_common::repository::errors::RepositoryError;

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
            page: Some(2),
        };

        let expected_hub = user.hub_id;
        let expected_per_page = DEFAULT_ITEMS_PER_PAGE;

        repo.expect_list_price_levels()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id, expected_hub);
                assert_eq!(query.search.as_deref(), Some("sil"));
                match &query.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 2);
                        assert_eq!(pagination.per_page, expected_per_page);
                    }
                    None => panic!("expected pagination to be set"),
                }
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

        let serialized = match serde_json::to_value(&data.price_levels) {
            Ok(value) => value,
            Err(err) => panic!("serialization failed: {err}"),
        };

        let page_value = match serialized.get("page") {
            Some(value) => value,
            None => panic!("missing page field"),
        };
        assert_eq!(page_value.as_u64(), Some(2));

        let items = match serialized.get("items") {
            Some(value) => match value.as_array() {
                Some(items) => items,
                None => panic!("items field is not an array"),
            },
            None => panic!("missing items field"),
        };
        assert_eq!(items.len(), 2);

        let first_name = items
            .first()
            .and_then(Value::as_object)
            .and_then(|map| map.get("name"))
            .and_then(Value::as_str);
        assert_eq!(first_name, Some("Silver"));
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
        fn get_customer_by_id(
            &self,
            id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_email(
            &self,
            email: &str,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_email(email, hub_id)
        }

        fn list_customers(
            &self,
            query: CustomerListQuery,
        ) -> pushkind_common::repository::errors::RepositoryResult<(usize, Vec<Customer>)> {
            self.customer_reader.list_customers(query)
        }
    }

    impl PriceLevelReader for ClientAssignmentRepo {
        fn get_price_level_by_id(
            &self,
            id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<PriceLevel>> {
            self.price_level_reader.get_price_level_by_id(id, hub_id)
        }

        fn list_price_levels(
            &self,
            query: PriceLevelListQuery,
        ) -> pushkind_common::repository::errors::RepositoryResult<(usize, Vec<PriceLevel>)>
        {
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

        assert_eq!(assignments.default_price_level_id, Some(10));
        assert_eq!(assignments.assignments.len(), 2);
        assert_eq!(
            assignments.assignments[0],
            ClientPriceLevelAssignment {
                customer_id: 1,
                price_level_id: Some(11),
            }
        );
        assert_eq!(
            assignments.assignments[1],
            ClientPriceLevelAssignment {
                customer_id: 2,
                price_level_id: None,
            }
        );
    }

    #[test]
    fn assign_price_level_to_client_requires_role() {
        let repo = MockCustomerWriter::new();
        let user = user_with_roles(&[]);
        let payload = AssignClientPriceLevelPayload {
            price_level_id: Some(5),
        };

        let result = assign_price_level_to_client(&repo, &user, 7, payload);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn assign_price_level_to_client_updates_assignment() {
        let mut repo = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let payload = AssignClientPriceLevelPayload {
            price_level_id: Some(8),
        };
        let hub_id = user.hub_id;

        repo.expect_assign_price_level_to_customers()
            .withf(move |target_hub, ids, price_level_id| {
                *target_hub == hub_id && ids == [7] && price_level_id == &Some(8)
            })
            .returning(|_, _, _| Ok(()));

        assign_price_level_to_client(&repo, &user, 7, payload).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_clears_assignment() {
        let mut repo = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let payload = AssignClientPriceLevelPayload {
            price_level_id: None,
        };
        let hub_id = user.hub_id;

        repo.expect_assign_price_level_to_customers()
            .withf(move |target_hub, ids, price_level_id| {
                *target_hub == hub_id && ids == [3] && price_level_id.is_none()
            })
            .returning(|_, _, _| Ok(()));

        assign_price_level_to_client(&repo, &user, 3, payload).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_propagates_form_errors() {
        let repo = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let payload = AssignClientPriceLevelPayload {
            price_level_id: Some(0),
        };

        let result = assign_price_level_to_client(&repo, &user, 3, payload);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(message.contains("invalid_price_level_id"));
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
