use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};

use crate::domain::order::OrderListQuery;
use crate::domain::types::HubId;
use crate::dto::main::{IndexPageData, IndexQuery};
use crate::repository::{OrderReader, UserReader, VendorUserReader};
use crate::services::{HubAccessScope, ServiceResult, resolve_hub_access};

/// Loads the orders list for the main index page.
pub fn load_index_page<R>(
    query: IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<IndexPageData>
where
    R: OrderReader + UserReader + VendorUserReader + ?Sized,
{
    let access = resolve_hub_access(user, repo)?;

    let page = query.page.unwrap_or(1);
    let hub_id = HubId::new(user.hub_id)?;
    let mut list_query = OrderListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let HubAccessScope::Vendor { vendor_id } = access {
        list_query = list_query.vendor_id(vendor_id);
    }

    if let Some(value) = query.search.as_ref() {
        list_query = list_query.search(value);
    }

    let (total, orders) = repo.list_orders(list_query)?;

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let orders = Paginated::new(orders, page, total_pages);

    Ok(IndexPageData {
        orders,
        search: query.search,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use pushkind_common::repository::errors::RepositoryResult;
    use pushkind_common::services::errors::ServiceError;
    use serde_json::Value;

    use crate::domain::order::{Order, OrderStatus};
    use crate::domain::types::{
        CurrencyCode, HubId, OrderId, OrderReference, PriceCents, UserEmail, UserId, UserName,
        VendorId,
    };
    use crate::domain::user::User;
    use crate::dto::main::IndexQuery;
    use crate::repository::UserListQuery;
    use crate::repository::mock::{MockOrderReader, MockUserReader, MockVendorUserReader};
    use crate::{SERVICE_ACCESS_ROLE, VENDOR_ACCESS_ROLE};

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_order(id: i32, hub_id: i32, reference: &str) -> Order {
        Order {
            id: OrderId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            customer_id: None,
            reference: Some(OrderReference::new(reference).unwrap()),
            status: OrderStatus::Pending,
            notes: None,
            total_cents: PriceCents::new(1000).unwrap(),
            currency: CurrencyCode::new("RUB").unwrap(),
            products: Vec::new(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 99,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    #[derive(Default)]
    struct IndexPageRepo {
        orders: MockOrderReader,
        user_reader: MockUserReader,
        vendor_user_reader: MockVendorUserReader,
    }

    impl OrderReader for IndexPageRepo {
        fn get_order_by_id(&self, id: OrderId, hub_id: HubId) -> RepositoryResult<Option<Order>> {
            self.orders.get_order_by_id(id, hub_id)
        }

        fn list_orders(
            &self,
            query: crate::domain::order::OrderListQuery,
        ) -> RepositoryResult<(usize, Vec<Order>)> {
            self.orders.list_orders(query)
        }
    }

    impl UserReader for IndexPageRepo {
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

    impl VendorUserReader for IndexPageRepo {
        fn get_vendor_for_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<VendorId>> {
            self.vendor_user_reader.get_vendor_for_user(user_id, hub_id)
        }
    }

    #[test]
    fn load_index_page_returns_unauthorized_when_role_missing() {
        let repo = IndexPageRepo::default();
        let user = user_with_roles(&[]);

        let result = load_index_page(IndexQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_index_page_returns_paginated_data() {
        let mut repo = IndexPageRepo::default();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: Some("alp".to_string()),
            page: Some(2),
        };

        let expected_hub = user.hub_id;

        repo.orders
            .expect_list_orders()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id.get(), expected_hub);
                assert_eq!(query.search.as_deref(), Some("alp"));
                match &query.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 2);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be set"),
                }
                true
            })
            .returning(move |_| {
                Ok((
                    45,
                    vec![
                        sample_order(1, expected_hub, "alpha-ref"),
                        sample_order(2, expected_hub, "beta-ref"),
                    ],
                ))
            });

        let result = load_index_page(query, &user, &repo);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.search.as_deref(), Some("alp"));

        let serialized = match serde_json::to_value(&data.orders) {
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

        let first_reference = items
            .first()
            .and_then(|item| item.as_object())
            .and_then(|map| map.get("reference"))
            .and_then(Value::as_str);
        assert_eq!(first_reference, Some("alpha-ref"));
    }

    #[test]
    fn load_index_page_scopes_vendor_access() {
        let mut repo = IndexPageRepo::default();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE, VENDOR_ACCESS_ROLE]);
        let expected_hub = user.hub_id;
        let hub_id = HubId::new(expected_hub).unwrap();
        let user_id = UserId::new(33).unwrap();
        let vendor_id = VendorId::new(4).unwrap();
        let user_record = User {
            id: user_id,
            hub_id,
            name: UserName::new("Vendor User").unwrap(),
            email: UserEmail::new(user.email.clone()).unwrap(),
        };

        repo.user_reader
            .expect_get_user_by_email()
            .times(1)
            .withf(move |email, hub| {
                email.as_str() == "user@example.com" && hub.get() == expected_hub
            })
            .returning(move |_, _| Ok(Some(user_record.clone())));

        repo.vendor_user_reader
            .expect_get_vendor_for_user()
            .times(1)
            .withf(move |id, hub| *id == user_id && hub.get() == expected_hub)
            .returning(move |_, _| Ok(Some(vendor_id)));

        repo.orders
            .expect_list_orders()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id.get(), expected_hub);
                assert_eq!(query.vendor_id, Some(vendor_id));
                true
            })
            .returning(|_| Ok((0, Vec::new())));

        let result = load_index_page(IndexQuery::default(), &user, &repo).expect("page result");

        let serialized = match serde_json::to_value(&result.orders) {
            Ok(value) => value,
            Err(err) => panic!("serialization failed: {err}"),
        };
        let items = match serialized.get("items") {
            Some(value) => match value.as_array() {
                Some(items) => items,
                None => panic!("items field is not an array"),
            },
            None => panic!("missing items field"),
        };
        assert!(items.is_empty());
    }
}
