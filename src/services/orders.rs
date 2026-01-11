use std::collections::{HashMap, HashSet};

use chrono::Utc;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::ensure_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::order::{Order, OrderProductApprovalUpdate};
use crate::domain::types::{HubId, OrderId, PriceCents, ProductId, ProductQuantity};
use crate::dto::orders::{OrderDetails, OrderProductApprovalPayload};
use crate::forms::orders::EditOrderForm;
use crate::repository::{CustomerReader, OrderReader, OrderWriter};
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
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let order_id = OrderId::new(order_id).map_err(|_| ServiceError::Internal)?;
    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;

    let order = repo
        .get_order_by_id(order_id, hub_id)
        .map_err(ServiceError::from)?;

    let order = order.ok_or(ServiceError::NotFound)?;

    let customer = match order.customer_id {
        Some(customer_id) => repo
            .get_customer_by_id(customer_id, hub_id)
            .map_err(ServiceError::from)?,
        None => None,
    };

    Ok(OrderDetails { order, customer })
}

/// Updates editable metadata for an existing order.
pub fn update_order<R>(
    repo: &R,
    user: &AuthenticatedUser,
    order_id: i32,
    form: EditOrderForm,
) -> ServiceResult<Order>
where
    R: OrderWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let order_id = OrderId::new(order_id).map_err(|_| ServiceError::Internal)?;
    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;

    let updates = form
        .into_update_order()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.update_order(order_id, hub_id, &updates)
        .map_err(ServiceError::from)
}

/// Updates approved quantities for order products and recalculates the order total.
pub fn update_order_product_approvals<R>(
    repo: &R,
    user: &AuthenticatedUser,
    order_id: i32,
    approvals: Vec<OrderProductApprovalPayload>,
) -> ServiceResult<OrderDetails>
where
    R: OrderReader + OrderWriter + CustomerReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    if approvals.is_empty() {
        return Err(ServiceError::Form(
            "Не выбраны позиции для обновления.".to_string(),
        ));
    }

    let order_id = OrderId::new(order_id).map_err(|_| ServiceError::Internal)?;
    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;

    let mut approvals_map: HashMap<ProductId, ProductQuantity> = HashMap::new();
    for payload in approvals {
        let product_id = ProductId::new(payload.product_id)
            .map_err(|_| ServiceError::Form("Неверный товар в запросе.".to_string()))?;
        let approved_quantity = ProductQuantity::new(payload.approved_quantity)
            .map_err(|_| ServiceError::Form("Количество должно быть положительным.".to_string()))?;
        approvals_map.insert(product_id, approved_quantity);
    }

    let order = repo
        .get_order_by_id(order_id, hub_id)
        .map_err(ServiceError::from)?
        .ok_or(ServiceError::NotFound)?;

    let mut updates: Vec<OrderProductApprovalUpdate> = Vec::new();
    let mut matched: HashSet<ProductId> = HashSet::new();
    let mut total_cents: i32 = 0;

    for product in &order.products {
        let approved_quantity = match product.product_id.and_then(|id| approvals_map.get(&id)) {
            Some(value) => *value,
            None => product.approved_quantity.unwrap_or(product.quantity),
        };

        let current_quantity = product.approved_quantity.unwrap_or(product.quantity);

        if product.price_cents.get() % current_quantity.get() != 0 {
            return Err(ServiceError::Internal);
        }

        let unit_price = product.price_cents.get() / current_quantity.get();
        let line_total = unit_price
            .checked_mul(approved_quantity.get())
            .ok_or(ServiceError::Internal)?;

        total_cents = total_cents
            .checked_add(line_total)
            .ok_or(ServiceError::Internal)?;

        if let Some(product_id) = product.product_id
            && let Some(updated_quantity) = approvals_map.get(&product_id)
        {
            matched.insert(product_id);
            let price_cents = PriceCents::new(line_total).map_err(|_| ServiceError::Internal)?;
            updates.push(OrderProductApprovalUpdate {
                product_id,
                approved_quantity: *updated_quantity,
                price_cents,
            });
        }
    }

    if matched.len() != approvals_map.len() {
        return Err(ServiceError::NotFound);
    }

    let total_cents = PriceCents::new(total_cents).map_err(|_| ServiceError::Internal)?;
    let updated_at = Utc::now().naive_utc();

    let order = repo
        .update_order_product_approvals(order_id, hub_id, &updates, total_cents, updated_at)
        .map_err(ServiceError::from)?;

    let customer = match order.customer_id {
        Some(customer_id) => repo
            .get_customer_by_id(customer_id, hub_id)
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
        PhoneNumber, PriceCents, ProductId, ProductName, ProductQuantity,
    };
    use crate::domain::{
        customer::Customer,
        order::{Order, OrderProduct, OrderStatus},
    };
    use crate::forms::orders::EditOrderForm;
    use crate::repository::mock::{MockCustomerReader, MockOrderReader, MockOrderWriter};
    use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

    #[derive(Default)]
    struct OrderServiceRepo {
        orders: MockOrderReader,
        customers: MockCustomerReader,
        order_writer: MockOrderWriter,
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
        fn get_customer_by_id(
            &self,
            id: CustomerId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.customers.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_phone(
            &self,
            phone: &PhoneNumber,
            hub_id: HubId,
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

    impl OrderWriter for OrderServiceRepo {
        fn create_order(
            &self,
            _new_order: &crate::domain::order::NewOrder,
        ) -> RepositoryResult<Order> {
            self.order_writer.create_order(_new_order)
        }

        fn update_order(
            &self,
            order_id: OrderId,
            hub_id: HubId,
            updates: &crate::domain::order::UpdateOrder,
        ) -> RepositoryResult<Order> {
            self.order_writer.update_order(order_id, hub_id, updates)
        }

        fn update_order_product_approvals(
            &self,
            order_id: OrderId,
            hub_id: HubId,
            updates: &[crate::domain::order::OrderProductApprovalUpdate],
            new_total_cents: PriceCents,
            updated_at: NaiveDateTime,
        ) -> RepositoryResult<Order> {
            self.order_writer.update_order_product_approvals(
                order_id,
                hub_id,
                updates,
                new_total_cents,
                updated_at,
            )
        }

        fn delete_order(&self, order_id: OrderId, hub_id: HubId) -> RepositoryResult<()> {
            self.order_writer.delete_order(order_id, hub_id)
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
                approved_quantity: Some(ProductQuantity::new(1).unwrap()),
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
            phone: PhoneNumber::new("+10000000000").unwrap(),
            price_level_id: None,
            public_id: None,
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

    fn sample_edit_form(status: &str) -> EditOrderForm {
        EditOrderForm {
            order_id: 5,
            status: status.to_string(),
            reference: Some(" REF ".to_string()),
            notes: Some(" notes ".to_string()),
            shipping_address: Some(" Address ".to_string()),
            consignee: Some(" Recipient ".to_string()),
            delivery_notes: Some(" Leave by door ".to_string()),
            payer: Some(" Payer ".to_string()),
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
            .withf(move |id, hub_id| id.get() == 11 && hub_id.get() == expected_hub)
            .returning(move |id, hub_id| Ok(Some(sample_customer(id.get(), hub_id.get()))));

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

    #[test]
    fn update_order_requires_role() {
        let repo = MockOrderWriter::new();
        let user = user_with_roles(&[]);
        let form = sample_edit_form("Pending");

        let result = update_order(&repo, &user, 5, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_order_calls_repository_with_sanitized_payload() {
        let mut repo = MockOrderWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;
        let form = sample_edit_form("Completed");

        repo.expect_update_order()
            .times(1)
            .withf(move |id, hub_id, updates| {
                id.get() == 5
                    && hub_id.get() == expected_hub
                    && updates.status == OrderStatus::Completed
                    && updates.reference.as_ref().map(|value| value.as_str()) == Some("REF")
                    && updates.notes.as_ref().map(|value| value.as_str()) == Some("notes")
            })
            .returning(move |_, _, _| Ok(sample_order(5, expected_hub, None)));

        let result = update_order(&repo, &user, 5, form);

        let order = match result {
            Ok(order) => order,
            Err(err) => panic!("expected update to succeed: {err}"),
        };

        assert_eq!(order.id.get(), 5);
        assert_eq!(order.hub_id.get(), expected_hub);
    }

    #[test]
    fn update_order_returns_not_found_when_missing() {
        let mut repo = MockOrderWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = sample_edit_form("Processing");

        repo.expect_update_order()
            .returning(|_, _, _| Err(RepositoryError::NotFound));

        let result = update_order(&repo, &user, 5, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn update_order_propagates_form_errors() {
        let repo = MockOrderWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditOrderForm {
            order_id: 5,
            status: "Unknown".to_string(),
            reference: None,
            notes: None,
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        };

        let result = update_order(&repo, &user, 5, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn update_order_product_approvals_rejects_missing_role() {
        let repo = OrderServiceRepo::new();
        let user = user_with_roles(&[]);

        let result = update_order_product_approvals(
            &repo,
            &user,
            1,
            vec![OrderProductApprovalPayload {
                product_id: 10,
                approved_quantity: 2,
            }],
        );

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_order_product_approvals_updates_totals_and_returns_details() {
        let mut repo = OrderServiceRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let order = sample_order(1, 7, Some(3));
        let updated_order = Order {
            total_cents: PriceCents::new(3000).unwrap(),
            products: vec![OrderProduct {
                approved_quantity: Some(ProductQuantity::new(2).unwrap()),
                price_cents: PriceCents::new(3000).unwrap(),
                ..order.products[0].clone()
            }],
            updated_at: fixed_datetime(),
            ..order.clone()
        };

        repo.orders
            .expect_get_order_by_id()
            .returning(move |id, hub_id| {
                assert_eq!(id.get(), 1);
                assert_eq!(hub_id.get(), 7);
                Ok(Some(order.clone()))
            });

        repo.order_writer
            .expect_update_order_product_approvals()
            .returning(move |order_id, hub_id, updates, total_cents, _| {
                assert_eq!(order_id.get(), 1);
                assert_eq!(hub_id.get(), 7);
                assert_eq!(updates.len(), 1);
                assert_eq!(updates[0].product_id.get(), 10);
                assert_eq!(updates[0].approved_quantity.get(), 2);
                assert_eq!(updates[0].price_cents.get(), 3000);
                assert_eq!(total_cents.get(), 3000);
                Ok(updated_order.clone())
            });

        repo.customers
            .expect_get_customer_by_id()
            .returning(|id, hub_id| {
                assert_eq!(id.get(), 3);
                assert_eq!(hub_id.get(), 7);
                Ok(Some(sample_customer(id.get(), hub_id.get())))
            });

        let result = update_order_product_approvals(
            &repo,
            &user,
            1,
            vec![OrderProductApprovalPayload {
                product_id: 10,
                approved_quantity: 2,
            }],
        )
        .expect("expected successful update");

        assert_eq!(result.order.total_cents.get(), 3000);
        assert_eq!(result.order.products[0].approved_quantity.unwrap().get(), 2);
        assert_eq!(result.order.products[0].price_cents.get(), 3000);
        assert!(result.customer.is_some());
    }

    #[test]
    fn update_order_product_approvals_uses_current_approved_quantity_for_unit_price() {
        let mut repo = OrderServiceRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let order = Order {
            id: OrderId::new(1).unwrap(),
            hub_id: HubId::new(7).unwrap(),
            customer_id: None,
            reference: None,
            status: OrderStatus::Pending,
            notes: None,
            total_cents: PriceCents::new(500).unwrap(),
            currency: CurrencyCode::new("RUB").unwrap(),
            products: vec![OrderProduct {
                product_id: Some(ProductId::new(10).unwrap()),
                name: ProductName::new("Sample").unwrap(),
                sku: None,
                description: None,
                price_cents: PriceCents::new(500).unwrap(),
                currency: CurrencyCode::new("RUB").unwrap(),
                quantity: ProductQuantity::new(10).unwrap(),
                approved_quantity: Some(ProductQuantity::new(5).unwrap()),
                default_price_cents: None,
            }],
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        };

        let updated_order = Order {
            total_cents: PriceCents::new(300).unwrap(),
            products: vec![OrderProduct {
                approved_quantity: Some(ProductQuantity::new(3).unwrap()),
                price_cents: PriceCents::new(300).unwrap(),
                ..order.products[0].clone()
            }],
            updated_at: fixed_datetime(),
            ..order.clone()
        };

        repo.orders
            .expect_get_order_by_id()
            .returning(move |id, hub_id| {
                assert_eq!(id.get(), 1);
                assert_eq!(hub_id.get(), 7);
                Ok(Some(order.clone()))
            });

        repo.order_writer
            .expect_update_order_product_approvals()
            .returning(move |order_id, hub_id, updates, total_cents, _| {
                assert_eq!(order_id.get(), 1);
                assert_eq!(hub_id.get(), 7);
                assert_eq!(updates.len(), 1);
                assert_eq!(updates[0].product_id.get(), 10);
                assert_eq!(updates[0].approved_quantity.get(), 3);
                assert_eq!(updates[0].price_cents.get(), 300);
                assert_eq!(total_cents.get(), 300);
                Ok(updated_order.clone())
            });

        let result = update_order_product_approvals(
            &repo,
            &user,
            1,
            vec![OrderProductApprovalPayload {
                product_id: 10,
                approved_quantity: 3,
            }],
        )
        .expect("expected successful update");

        assert_eq!(result.order.total_cents.get(), 300);
        assert_eq!(result.order.products[0].approved_quantity.unwrap().get(), 3);
        assert_eq!(result.order.products[0].price_cents.get(), 300);
        assert!(result.customer.is_none());
    }

    #[test]
    fn update_order_product_approvals_errors_for_unknown_line() {
        let mut repo = OrderServiceRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let order = sample_order(1, 7, None);

        repo.orders
            .expect_get_order_by_id()
            .returning(move |_, _| Ok(Some(order.clone())));

        let result = update_order_product_approvals(
            &repo,
            &user,
            1,
            vec![OrderProductApprovalPayload {
                product_id: 99,
                approved_quantity: 1,
            }],
        );

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }
}
