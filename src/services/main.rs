use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::ensure_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::order::OrderListQuery;
use crate::domain::types::HubId;
use crate::dto::main::{IndexPageData, IndexQuery};
use crate::repository::OrderReader;
use crate::services::ServiceResult;

/// Loads the orders list for the main index page.
pub fn load_index_page<R>(
    query: IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<IndexPageData>
where
    R: OrderReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let page = query.page.unwrap_or(1);
    let hub_id = HubId::new(user.hub_id)?;
    let mut list_query = OrderListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

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
    use pushkind_common::services::errors::ServiceError;
    use serde_json::Value;

    use crate::SERVICE_ACCESS_ROLE;
    use crate::domain::order::{Order, OrderStatus};
    use crate::domain::types::{CurrencyCode, HubId, OrderId, OrderReference, PriceCents};
    use crate::dto::main::IndexQuery;
    use crate::repository::mock::MockOrderReader;

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

    #[test]
    fn load_index_page_returns_unauthorized_when_role_missing() {
        let repo = MockOrderReader::new();
        let user = user_with_roles(&[]);

        let result = load_index_page(IndexQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_index_page_returns_paginated_data() {
        let mut repo = MockOrderReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: Some("alp".to_string()),
            page: Some(2),
        };

        let expected_hub = user.hub_id;

        repo.expect_list_orders()
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
}
