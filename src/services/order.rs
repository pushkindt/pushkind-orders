use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::order::Order;
use crate::repository::OrderReader;
use crate::services::{ServiceError, ServiceResult};

/// Loads a single order owned by the authenticated user's hub.
pub fn load_order_details<R>(
    repo: &R,
    user: &AuthenticatedUser,
    order_id: i32,
) -> ServiceResult<Order>
where
    R: OrderReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let order = repo
        .get_order_by_id(order_id, user.hub_id)
        .map_err(ServiceError::from)?;

    order.ok_or(ServiceError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::domain::order::{OrderProduct, OrderStatus};
    use crate::repository::mock::MockOrderReader;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_order(id: i32, hub_id: i32) -> Order {
        Order {
            id,
            hub_id,
            customer_id: None,
            reference: Some(format!("ORD-{id}")),
            status: OrderStatus::Pending,
            notes: Some("Notes".to_string()),
            total_cents: 1500,
            currency: "RUB".to_string(),
            products: vec![OrderProduct {
                product_id: Some(10),
                name: "Sample".to_string(),
                sku: None,
                description: None,
                price_cents: 1500,
                currency: "RUB".to_string(),
                quantity: 1,
            }],
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

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
    fn load_order_details_returns_unauthorized_without_role() {
        let repo = MockOrderReader::new();
        let user = user_with_roles(&[]);

        let result = load_order_details(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_order_details_returns_not_found_for_missing_order() {
        let mut repo = MockOrderReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;

        repo.expect_get_order_by_id()
            .times(1)
            .withf(move |id, hub_id| *id == 5 && *hub_id == expected_hub)
            .returning(|_, _| Ok(None));

        let result = load_order_details(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_order_details_returns_order_when_present() {
        let mut repo = MockOrderReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;

        repo.expect_get_order_by_id()
            .times(1)
            .withf(move |id, hub_id| *id == 3 && *hub_id == expected_hub)
            .returning(move |id, hub_id| Ok(Some(sample_order(id, hub_id))));

        let result = load_order_details(&repo, &user, 3);

        let order = match result {
            Ok(order) => order,
            Err(err) => panic!("expected order, got error: {err}"),
        };

        assert_eq!(order.id, 3);
        assert_eq!(order.hub_id, expected_hub);
    }
}
