use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::types::{HubId, OrderId};
use crate::dto::orders::OrderDetails;
use crate::repository::{CustomerReader, OrderReader};
use crate::services::{ServiceError, ServiceResult};

/// Loads a single order owned by the authenticated user's hub.
pub fn load_order_details<R>(
    repo: &R,
    user: &AuthenticatedUser,
    order_id: i32,
) -> ServiceResult<OrderDetails>
where
    R: OrderReader + CustomerReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let order_id = OrderId::new(order_id).map_err(|_| ServiceError::Internal)?;
    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;

    let order = repo
        .get_order_by_id(order_id, hub_id)
        .map_err(ServiceError::from)?;

    let order = order.ok_or(ServiceError::NotFound)?;

    let customer = match order.customer_id {
        Some(customer_id) => repo
            .get_customer_by_id(customer_id.get(), hub_id.get())
            .map_err(ServiceError::from)?,
        None => None,
    };

    Ok(OrderDetails { order, customer })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::domain::types::{
        CurrencyCode, CustomerId, CustomerName, HubId, OrderId, OrderNotes, OrderReference,
        PhoneNumber, PriceCents, ProductId, ProductName, ProductQuantity, UserEmail,
    };
    use crate::domain::{
        customer::Customer,
        order::{Order, OrderProduct, OrderStatus},
    };
    use crate::repository::mock::{MockCustomerReader, MockOrderReader};
    use pushkind_common::repository::errors::RepositoryResult;

    #[derive(Default)]
    struct OrderServiceRepo {
        orders: MockOrderReader,
        customers: MockCustomerReader,
    }

    impl OrderServiceRepo {
        fn new() -> Self {
            Self::default()
        }
    }

    impl OrderReader for OrderServiceRepo {
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

    impl CustomerReader for OrderServiceRepo {
        fn get_customer_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Customer>> {
            self.customers.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_email(
            &self,
            email: &str,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>> {
            self.customers.get_customer_by_email(email, hub_id)
        }

        fn get_customer_by_phone(
            &self,
            phone: &str,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>> {
            self.customers.get_customer_by_phone(phone, hub_id)
        }

        fn list_customers(
            &self,
            query: crate::domain::customer::CustomerListQuery,
        ) -> RepositoryResult<(usize, Vec<Customer>)> {
            self.customers.list_customers(query)
        }
    }

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_order(id: i32, hub_id: i32, customer_id: Option<i32>) -> Order {
        Order {
            id: OrderId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            customer_id: customer_id.map(|id| CustomerId::new(id).unwrap()),
            reference: Some(OrderReference::new(format!("ORD-{id}")).unwrap()),
            status: OrderStatus::Pending,
            notes: Some(OrderNotes::new("Notes").unwrap()),
            total_cents: PriceCents::new(1500).unwrap(),
            currency: CurrencyCode::new("RUB").unwrap(),
            products: vec![OrderProduct {
                product_id: Some(ProductId::new(10).unwrap()),
                name: ProductName::new("Sample").unwrap(),
                sku: None,
                description: None,
                price_cents: PriceCents::new(1500).unwrap(),
                currency: CurrencyCode::new("RUB").unwrap(),
                quantity: ProductQuantity::new(1).unwrap(),
                default_price_cents: None,
            }],
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        }
    }

    fn sample_customer(id: i32, hub_id: i32) -> Customer {
        Customer {
            id: CustomerId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: CustomerName::new("Sample Customer").unwrap(),
            email: Some(UserEmail::new("customer@example.com").unwrap()),
            phone: PhoneNumber::new("+10000000000").unwrap(),
            price_level_id: None,
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
        let repo = OrderServiceRepo::new();
        let user = user_with_roles(&[]);

        let result = load_order_details(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_order_details_returns_not_found_for_missing_order() {
        let mut repo = OrderServiceRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;

        repo.orders
            .expect_get_order_by_id()
            .times(1)
            .withf(move |id, hub_id| id.get() == 5 && hub_id.get() == expected_hub)
            .returning(|_, _| Ok(None));

        let result = load_order_details(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_order_details_returns_order_when_present() {
        let mut repo = OrderServiceRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;

        repo.orders
            .expect_get_order_by_id()
            .times(1)
            .withf(move |id, hub_id| id.get() == 3 && hub_id.get() == expected_hub)
            .returning(move |id, hub_id| Ok(Some(sample_order(id.get(), hub_id.get(), None))));

        let result = load_order_details(&repo, &user, 3);

        let details = match result {
            Ok(details) => details,
            Err(err) => panic!("expected order details, got error: {err}"),
        };

        assert_eq!(details.order.id.get(), 3);
        assert_eq!(details.order.hub_id.get(), expected_hub);
        assert!(details.customer.is_none());
    }

    #[test]
    fn load_order_details_includes_customer_when_present() {
        let mut repo = OrderServiceRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;

        repo.orders
            .expect_get_order_by_id()
            .times(1)
            .withf(move |id, hub_id| id.get() == 4 && hub_id.get() == expected_hub)
            .returning(move |id, hub_id| Ok(Some(sample_order(id.get(), hub_id.get(), Some(11)))));

        repo.customers
            .expect_get_customer_by_id()
            .times(1)
            .withf(move |id, hub_id| *id == 11 && *hub_id == expected_hub)
            .returning(move |id, hub_id| Ok(Some(sample_customer(id, hub_id))));

        let result = load_order_details(&repo, &user, 4);

        let details = match result {
            Ok(details) => details,
            Err(err) => panic!("expected order details, got error: {err}"),
        };

        assert_eq!(details.order.id.get(), 4);
        let customer = details.customer.expect("expected customer details");
        assert_eq!(customer.id.get(), 11);
        assert_eq!(customer.hub_id.get(), expected_hub);
    }
}
