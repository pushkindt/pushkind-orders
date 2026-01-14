use chrono::{Duration, Utc};
use log::info;
use pushkind_common::{
    models::sms::zmq::ZMQSendSmsMessage,
    pagination::DEFAULT_ITEMS_PER_PAGE,
    zmq::{ZmqSenderExt, ZmqSenderTrait},
};
use pushkind_crm::models::zmq::ZmqClientMessage;
use rand::Rng;

use crate::domain::{
    category::CategoryTreeQuery,
    customer::{Customer, NewCustomer},
    order::{NewOrder, Order, OrderListQuery, OrderProduct, OrderStatus, UpdateOrder},
    price_level::PriceLevelListQuery,
    store_otp::NewStoreOtp,
    tag::TagListQuery,
    types::{CategoryId, HubId, OrderId, PhoneNumber, ProductId, ProductQuantity},
};
pub use crate::dto::store::{
    StoreCategory, StoreCategoryFilters, StoreOrder, StoreOrderProduct, StoreOtpAcceptResponse,
    StoreOtpVerifyResponse, StoreProduct, StoreProductFilters, StoreTag,
};
use crate::forms::store::{
    StoreOrderLinePayload, StoreOrderUpdateValues, StoreOtpRequestPayload, StoreOtpVerifyPayload,
    validate_store_order_lines,
};
use crate::repository::{
    CategoryReader, CustomerReader, CustomerWriter, OrderReader, OrderWriter, PriceLevelReader,
    ProductReader, StoreOtpRepository, TagReader,
};
use crate::services::{ServiceError, ServiceResult};

const OTP_EXPIRY_MINUTES: i64 = 10;
const OTP_THROTTLE_MINUTES: i64 = 2;
const OTP_THROTTLE_MESSAGE: &str = "Please wait before requesting another OTP";
const OTP_INVALID_MESSAGE: &str = "Invalid or expired OTP";

/// Accepts an OTP request for the given hub.
pub async fn request_store_otp<R>(
    hub_id: i32,
    payload: StoreOtpRequestPayload,
    repo: &R,
    zmq_sender: &impl ZmqSenderTrait,
    sms_sender: &str,
) -> ServiceResult<StoreOtpAcceptResponse>
where
    R: StoreOtpRepository,
{
    let request = payload.into_request()?;
    let hub_id = HubId::new(hub_id)?;
    let phone = PhoneNumber::new(request.phone.clone())?;

    let now = Utc::now().naive_utc();

    if let Some(existing) = repo.get_store_otp(hub_id, &phone)?
        && existing.last_sent_at + Duration::minutes(OTP_THROTTLE_MINUTES) > now
    {
        return Err(ServiceError::Form(OTP_THROTTLE_MESSAGE.to_string()));
    }

    let code = format!("{:06}", rand::rng().random_range(0..1_000_000));
    let expires_at = now + Duration::minutes(OTP_EXPIRY_MINUTES);
    let otp_payload =
        NewStoreOtp::try_new(hub_id.get(), phone.as_str(), code.clone(), expires_at, now)?;

    repo.upsert_store_otp(&otp_payload)?;

    let zmq_message = ZMQSendSmsMessage {
        sender_id: sms_sender.to_string(),
        phone_number: phone.as_str().to_string(),
        message: format!("Your OTP is {code}"),
    };

    zmq_sender.send_json(&zmq_message).await?;

    info!(
        "Storefront OTP request accepted for hub {hub_id} and phone {}",
        phone.as_str()
    );

    Ok(StoreOtpAcceptResponse { success: true })
}

/// Verifies an OTP submission for the given hub.
pub async fn verify_store_otp<R>(
    hub_id: i32,
    payload: StoreOtpVerifyPayload,
    repo: &R,
    zmq_sender: &impl ZmqSenderTrait,
) -> ServiceResult<StoreOtpVerifyResponse>
where
    R: CustomerReader + CustomerWriter + StoreOtpRepository,
{
    let request = payload.into_request()?;
    let hub_id = HubId::new(hub_id)?;
    let phone = PhoneNumber::new(request.phone.clone())?;

    let now = Utc::now().naive_utc();

    let record = repo
        .get_store_otp(hub_id, &phone)?
        .ok_or_else(|| ServiceError::Form(OTP_INVALID_MESSAGE.to_string()))?;

    if record.code.as_str() != request.otp {
        return Err(ServiceError::Form(OTP_INVALID_MESSAGE.to_string()));
    }

    if record.expires_at <= now {
        return Err(ServiceError::Form(OTP_INVALID_MESSAGE.to_string()));
    }

    repo.delete_store_otp(hub_id, &phone)?;

    let customer = match repo.get_customer_by_phone(&phone, hub_id)? {
        Some(customer) => customer,
        None => {
            let new_customer = NewCustomer::try_new(hub_id.get(), phone.as_str(), phone.as_str())?;

            repo.create_customer(&new_customer)?
        }
    };

    let message = ZmqClientMessage {
        hub_id: hub_id.get(),
        name: customer.name.as_str().to_string(),
        email: None,
        phone: Some(customer.phone.as_str().to_string()),
        fields: None,
    };

    zmq_sender.send_json(&message).await?;

    info!(
        "Storefront OTP verification for hub {hub_id} with phone {} and code {}",
        phone.as_str(),
        request.otp
    );

    Ok(StoreOtpVerifyResponse {
        success: true,
        customer,
    })
}
/// Resolves the default price level ID for a hub.
fn resolve_default_price_level_id<R>(hub_id: i32, repo: &R) -> ServiceResult<Option<i32>>
where
    R: PriceLevelReader + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;
    let (_, price_levels) = repo.list_price_levels(PriceLevelListQuery::new(hub_id))?;

    Ok(price_levels
        .into_iter()
        .find(|level| level.is_default)
        .map(|level| level.id.get()))
}

/// Create a storefront order for the authenticated customer.
pub fn create_store_order<R>(
    hub_id: i32,
    payloads: Vec<StoreOrderLinePayload>,
    customer: &Customer,
    repo: &R,
) -> ServiceResult<Order>
where
    R: ProductReader + PriceLevelReader + OrderWriter + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;
    let items = validate_store_order_lines(payloads)?;

    let default_price_level_id = resolve_default_price_level_id(hub_id.get(), repo)?;
    let customer_price_level_id = customer.price_level_id.map(|id| id.get());

    let mut currency: Option<String> = None;
    let mut total_cents: i32 = 0;
    let mut products: Vec<OrderProduct> = Vec::new();

    for item in items {
        if item.quantity <= 0 {
            return Err(ServiceError::Form("quantity must be positive".to_string()));
        }

        let product_id = ProductId::new(item.product_id)
            .map_err(|_| ServiceError::Form("product not found".to_string()))?;
        let product = repo
            .get_product_by_id(product_id, hub_id)?
            .filter(|product| !product.is_archived)
            .ok_or_else(|| ServiceError::Form("product not found".to_string()))?;

        let price_cents = if let Some(customer_price_level_id) = customer_price_level_id {
            StoreProduct::resolve_price_cents(
                &product.price_levels,
                Some(customer_price_level_id),
                None,
            )
        } else {
            StoreProduct::resolve_price_cents(&product.price_levels, None, default_price_level_id)
        }
        .ok_or_else(|| ServiceError::Form("price unavailable".to_string()))?;

        let default_price_cents =
            StoreProduct::resolve_price_cents(&product.price_levels, None, default_price_level_id);

        let product_currency = product.currency.as_str().to_string();
        match &currency {
            Some(expected) if expected != &product_currency => {
                return Err(ServiceError::Form(
                    "mixed currencies are not allowed".to_string(),
                ));
            }
            None => currency = Some(product_currency.clone()),
            _ => {}
        }

        let approved_quantity = ProductQuantity::new(item.quantity)
            .map_err(|_| ServiceError::Form("quantity must be positive".to_string()))?;

        let line_total = price_cents
            .checked_mul(approved_quantity.get())
            .ok_or_else(|| ServiceError::Form("invalid order total".to_string()))?;

        total_cents = total_cents
            .checked_add(line_total)
            .ok_or_else(|| ServiceError::Form("invalid order total".to_string()))?;

        let mut order_product = OrderProduct::try_new(
            product.name.as_str(),
            line_total,
            product_currency.clone(),
            approved_quantity.get(),
            default_price_cents,
        )?
        .with_product_id(product.id)
        .with_approved_quantity(approved_quantity);

        if let Some(sku) = &product.sku {
            order_product = order_product.with_sku(sku.clone());
        }

        if let Some(description) = &product.description {
            order_product = order_product.with_description(description.clone());
        }

        products.push(order_product);
    }

    let currency = currency.unwrap_or_default();

    let new_order = NewOrder::try_new(hub_id.get(), total_cents, currency)?
        .with_customer_id(customer.id)
        .with_status(OrderStatus::Pending)
        .with_products(products);

    Ok(repo.create_order(&new_order)?)
}

/// Load orders placed by a storefront customer for the provided hub.
pub fn list_store_orders<R>(
    hub_id: i32,
    page: Option<usize>,
    customer: &Customer,
    repo: &R,
) -> ServiceResult<Vec<StoreOrder>>
where
    R: OrderReader + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;
    let mut query = OrderListQuery::new(hub_id).customer_id(customer.id);

    if let Some(page) = page.filter(|page| *page > 0) {
        query = query.paginate(page, DEFAULT_ITEMS_PER_PAGE);
    }

    let (_, orders) = repo.list_orders(query)?;

    Ok(orders.into_iter().map(StoreOrder::from).collect())
}

/// Apply storefront-provided metadata to an existing customer order.
pub fn update_store_order<R>(
    hub_id: i32,
    order_id: i32,
    values: StoreOrderUpdateValues,
    customer: &Customer,
    repo: &R,
) -> ServiceResult<StoreOrder>
where
    R: OrderReader + OrderWriter + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;
    let order_id = OrderId::new(order_id)?;

    let order = repo
        .get_order_by_id(order_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    if order.customer_id != Some(customer.id) {
        return Err(ServiceError::Unauthorized);
    }

    let shipping_address = merge_updates(values.shipping_address, order.shipping_address.clone());
    let consignee = merge_updates(values.consignee, order.consignee.clone());
    let delivery_notes = merge_updates(values.delivery_notes, order.delivery_notes.clone());
    let payer = merge_updates(values.payer, order.payer.clone());

    let updates = UpdateOrder {
        status: order.status,
        notes: order.notes.clone(),
        reference: order.reference.clone(),
        updated_at: Utc::now().naive_utc(),
        shipping_address,
        consignee,
        delivery_notes,
        payer,
    };

    let updated_order = repo.update_order(order_id, hub_id, &updates)?;

    Ok(StoreOrder::from(updated_order))
}

fn merge_updates<T>(incoming: Option<Option<T>>, existing: Option<T>) -> Option<T> {
    match incoming {
        None => existing,
        Some(value) => value,
    }
}

/// Load categories available to a storefront for the provided hub.
pub fn load_store_categories<R>(
    hub_id: i32,
    filters: StoreCategoryFilters,
    repo: &R,
) -> ServiceResult<Vec<StoreCategory>>
where
    R: CategoryReader + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;
    let query = CategoryTreeQuery::new(hub_id);
    let categories = repo.list_categories(query)?.1;

    let parent_id = filters.parent_id.and_then(|id| CategoryId::new(id).ok());
    let filtered = categories
        .into_iter()
        .filter(|category| !category.is_archived)
        .filter(|category| match parent_id {
            Some(parent_id) => category.parent_id == Some(parent_id),
            None => category.parent_id.is_none(),
        })
        .map(StoreCategory::from)
        .collect();

    Ok(filtered)
}

/// Load products available to a storefront for the provided hub.
pub fn load_store_products<R>(
    hub_id: i32,
    filters: StoreProductFilters,
    store_customer: Option<&Customer>,
    repo: &R,
) -> ServiceResult<Vec<StoreProduct>>
where
    R: ProductReader + PriceLevelReader + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;

    let default_price_level_id = resolve_default_price_level_id(hub_id.get(), repo)?;
    let customer_price_level = store_customer.and_then(|customer| customer.price_level_id);
    let customer_price_level_id = customer_price_level.map(|id| id.get());

    let mut query = filters.into_query(hub_id);
    if let Some(price_level_id) = customer_price_level {
        query = query.with_price_level_id(price_level_id);
    }

    let products = repo.list_products(query)?.1;

    let filtered = products
        .into_iter()
        .filter(|product| !product.is_archived)
        .filter(|product| match customer_price_level_id {
            Some(level_id) => product
                .price_levels
                .iter()
                .any(|rate| rate.price_level_id.get() == level_id),
            None => true,
        })
        .map(|product| {
            StoreProduct::from_domain(product, customer_price_level_id, default_price_level_id)
        })
        .collect();

    Ok(filtered)
}

/// Load a single product available to a storefront for the provided hub.
pub fn load_store_product<R>(
    hub_id: i32,
    product_id: i32,
    store_customer: Option<&Customer>,
    repo: &R,
) -> ServiceResult<Option<StoreProduct>>
where
    R: ProductReader + PriceLevelReader + ?Sized,
{
    let hub_id = HubId::new(hub_id)?;
    let product_id = ProductId::new(product_id)?;

    let product = repo.get_product_by_id(product_id, hub_id)?;

    let product = match product {
        Some(product) if !product.is_archived => product,
        _ => return Ok(None),
    };

    let default_price_level_id = resolve_default_price_level_id(hub_id.get(), repo)?;
    let customer_price_level_id =
        store_customer.and_then(|customer| customer.price_level_id.map(|id| id.get()));

    if let Some(level_id) = customer_price_level_id
        && !product
            .price_levels
            .iter()
            .any(|rate| rate.price_level_id.get() == level_id)
    {
        return Ok(None);
    }

    Ok(Some(StoreProduct::from_domain(
        product,
        customer_price_level_id,
        default_price_level_id,
    )))
}

/// Load tags available to a storefront for the provided hub.
pub fn load_store_tags<R>(hub_id: i32, repo: &R) -> ServiceResult<Vec<StoreTag>>
where
    R: TagReader + ?Sized,
{
    let tags = repo.list_tags(TagListQuery::try_new(hub_id)?)?.1;

    let mut formatted: Vec<StoreTag> = tags.into_iter().map(StoreTag::from).collect();
    formatted.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(formatted)
}

/// Ensures the customer stored in the store session still exists in the database.
pub fn load_store_session_customer<R>(
    session_customer: &Customer,
    repo: &R,
) -> ServiceResult<Customer>
where
    R: CustomerReader + ?Sized,
{
    repo.get_customer_by_id(session_customer.id, session_customer.hub_id)?
        .ok_or(ServiceError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        CategoryId, CategoryName, CurrencyCode, CustomerId, CustomerName, HubId, ImageUrl,
        OrderConsignee, OrderDeliveryNotes, OrderId, OrderPayer, OrderShippingAddress, PriceCents,
        PriceLevelId, PriceLevelName, ProductDescription, ProductId, ProductName,
        ProductPriceLevelRateId, ProductSku, ProductUnits, TagId, TagName,
    };
    use crate::domain::{
        category::Category,
        customer::{Customer, CustomerListQuery},
        order::{Order as DomainOrder, OrderStatus as DomainOrderStatus, UpdateOrder},
        price_level::{PriceLevel, PriceLevelListQuery},
        product::{Product, ProductListQuery},
        product_price_level::ProductPriceLevelRate,
        store_otp::{NewStoreOtp as DomainNewStoreOtp, StoreOtp as DomainStoreOtp},
        tag::Tag,
        types::{OtpCode, PhoneNumber},
    };
    use crate::dto::store::{StoreCategoryFilters, StoreProductFilters};
    use crate::forms::store::{
        StoreOrderLinePayload, StoreOrderUpdateValues, StoreOtpRequestPayload,
        StoreOtpVerifyPayload,
    };
    use crate::repository::mock::{
        MockCategoryReader, MockCustomerReader, MockCustomerWriter, MockOrderReader,
        MockOrderWriter, MockPriceLevelReader, MockProductReader, MockStoreOtpRepository,
    };
    use crate::repository::{
        CustomerReader, CustomerWriter, OrderReader, OrderWriter, StoreOtpRepository,
    };
    use chrono::NaiveDateTime;

    use pushkind_common::repository::errors::RepositoryResult;
    use pushkind_common::zmq::{SendFuture, ZmqSenderError, ZmqSenderTrait};
    use pushkind_crm::models::zmq::ZmqClientMessage;
    use serde_json;
    use std::sync::{Arc, Mutex};

    struct MockStoreOrderRepo {
        product_reader: MockProductReader,
        price_level_reader: MockPriceLevelReader,
        order_writer: MockOrderWriter,
    }

    impl MockStoreOrderRepo {
        fn new() -> Self {
            Self {
                product_reader: MockProductReader::new(),
                price_level_reader: MockPriceLevelReader::new(),
                order_writer: MockOrderWriter::new(),
            }
        }
    }

    impl ProductReader for MockStoreOrderRepo {
        fn get_product_by_id(
            &self,
            id: ProductId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Product>> {
            self.product_reader.get_product_by_id(id, hub_id)
        }

        fn list_products(
            &self,
            query: ProductListQuery,
        ) -> RepositoryResult<(usize, Vec<Product>)> {
            self.product_reader.list_products(query)
        }
    }

    impl PriceLevelReader for MockStoreOrderRepo {
        fn get_price_level_by_id(
            &self,
            id: PriceLevelId,
            hub_id: HubId,
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

    impl OrderWriter for MockStoreOrderRepo {
        fn create_order(&self, new_order: &NewOrder) -> RepositoryResult<Order> {
            self.order_writer.create_order(new_order)
        }

        fn update_order(
            &self,
            order_id: OrderId,
            hub_id: HubId,
            updates: &UpdateOrder,
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

    fn sample_timestamp() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn sample_price_level(id: i32, is_default: bool) -> PriceLevel {
        PriceLevel {
            id: PriceLevelId::new(id).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: PriceLevelName::new(format!("Level {id}")).unwrap(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            is_default,
        }
    }

    fn sample_product(
        id: i32,
        hub_id: i32,
        price_level_id: i32,
        price_cents: i32,
        currency: &str,
    ) -> Product {
        use crate::domain::types::*;
        Product {
            id: ProductId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: ProductName::new(format!("Product {id}")).unwrap(),
            sku: Some(ProductSku::new(format!("SKU{id}")).unwrap()),
            description: Some(ProductDescription::new(format!("Description {id}")).unwrap()),
            units: None,
            currency: CurrencyCode::new(currency).unwrap(),
            is_archived: false,
            category_id: None,
            price_levels: vec![ProductPriceLevelRate {
                id: ProductPriceLevelRateId::new(id).unwrap(),
                product_id: ProductId::new(id).unwrap(),
                price_level_id: PriceLevelId::new(price_level_id).unwrap(),
                price_cents: PriceCents::new(price_cents).unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
            }],
            tags: Vec::new(),
            image_urls: Vec::new(),
            amount: None,
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
        }
    }

    fn tag(id: i32, hub_id: i32, name: &str) -> Tag {
        Tag {
            id: TagId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: TagName::new(name).unwrap(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
        }
    }

    fn phone_number(value: &str) -> PhoneNumber {
        PhoneNumber::new(value).unwrap()
    }

    fn otp_code(value: &str) -> OtpCode {
        OtpCode::new(value).unwrap()
    }

    struct MockOtpRequestRepo {
        customer_reader: MockCustomerReader,
        customer_writer: MockCustomerWriter,
        otp_repository: MockStoreOtpRepository,
    }

    impl MockOtpRequestRepo {
        fn new() -> Self {
            Self {
                customer_reader: MockCustomerReader::new(),
                customer_writer: MockCustomerWriter::new(),
                otp_repository: MockStoreOtpRepository::new(),
            }
        }
    }

    #[test]
    fn create_store_order_creates_pending_order() {
        let mut repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: None,
            public_id: None,
        };

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(|query| {
                assert_eq!(query.hub_id.get(), 1);
                Ok((1, vec![sample_price_level(2, true)]))
            });

        repo.product_reader
            .expect_get_product_by_id()
            .returning(|product_id, hub_id| match product_id.get() {
                1 => Ok(Some(sample_product(1, hub_id.get(), 2, 500, "USD"))),
                2 => Ok(Some(sample_product(2, hub_id.get(), 2, 300, "USD"))),
                _ => Ok(None),
            });

        repo.order_writer
            .expect_create_order()
            .times(1)
            .withf(|new_order| {
                assert_eq!(new_order.hub_id.get(), 1);
                assert_eq!(new_order.customer_id.map(|id| id.get()), Some(10));
                assert_eq!(new_order.total_cents.get(), 1300);
                assert_eq!(new_order.currency.as_str(), "USD");
                assert_eq!(new_order.status, DomainOrderStatus::Pending);
                assert_eq!(new_order.products.len(), 2);
                assert_eq!(new_order.products[0].product_id.map(|id| id.get()), Some(1));
                assert_eq!(new_order.products[0].price_cents.get(), 1000);
                assert_eq!(
                    new_order.products[0]
                        .approved_quantity
                        .map(|quantity| quantity.get()),
                    Some(2)
                );
                true
            })
            .returning(|new_order| {
                use crate::domain::types::OrderId;
                Ok(DomainOrder {
                    id: OrderId::new(99).unwrap(),
                    hub_id: new_order.hub_id,
                    customer_id: new_order.customer_id,
                    reference: None,
                    status: DomainOrderStatus::Pending,
                    notes: None,
                    total_cents: new_order.total_cents,
                    currency: new_order.currency.clone(),
                    products: new_order.products.clone(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                    shipping_address: None,
                    consignee: None,
                    delivery_notes: None,
                    payer: None,
                })
            });

        let payload = vec![
            StoreOrderLinePayload {
                product_id: 1,
                quantity: 2,
            },
            StoreOrderLinePayload {
                product_id: 2,
                quantity: 1,
            },
        ];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(result.is_ok());
    }

    #[test]
    fn create_store_order_uses_customer_price_level_when_present() {
        let mut repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: Some(PriceLevelId::new(11).unwrap()),
            public_id: None,
        };

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(|query| {
                assert_eq!(query.hub_id.get(), 1);
                Ok((
                    2,
                    vec![sample_price_level(10, true), sample_price_level(11, false)],
                ))
            });

        repo.product_reader
            .expect_get_product_by_id()
            .returning(|product_id, hub_id| {
                let mut product = sample_product(product_id.get(), hub_id.get(), 10, 450, "USD");
                product.price_levels.push(ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(999).unwrap(),
                    product_id: product.id,
                    price_level_id: PriceLevelId::new(11).unwrap(),
                    price_cents: PriceCents::new(500).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                });
                Ok(Some(product))
            });

        repo.order_writer
            .expect_create_order()
            .times(1)
            .withf(|new_order| {
                assert_eq!(new_order.hub_id.get(), 1);
                assert_eq!(new_order.customer_id.map(|id| id.get()), Some(10));
                assert_eq!(new_order.total_cents.get(), 500);
                assert_eq!(new_order.currency.as_str(), "USD");
                assert_eq!(new_order.products.len(), 1);
                assert_eq!(new_order.products[0].price_cents.get(), 500);
                assert_eq!(
                    new_order.products[0]
                        .default_price_cents
                        .map(|price| price.get()),
                    Some(450)
                );
                true
            })
            .returning(|new_order| {
                use crate::domain::types::OrderId;
                Ok(DomainOrder {
                    id: OrderId::new(99).unwrap(),
                    hub_id: new_order.hub_id,
                    customer_id: new_order.customer_id,
                    reference: None,
                    status: DomainOrderStatus::Pending,
                    notes: None,
                    total_cents: new_order.total_cents,
                    currency: new_order.currency.clone(),
                    products: new_order.products.clone(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                    shipping_address: None,
                    consignee: None,
                    delivery_notes: None,
                    payer: None,
                })
            });

        let payload = vec![StoreOrderLinePayload {
            product_id: 1,
            quantity: 1,
        }];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(result.is_ok());
    }

    #[test]
    fn create_store_order_rejects_missing_customer_price_level_rate() {
        let mut repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: Some(PriceLevelId::new(11).unwrap()),
            public_id: None,
        };

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(|query| {
                assert_eq!(query.hub_id.get(), 1);
                Ok((
                    2,
                    vec![sample_price_level(10, true), sample_price_level(11, false)],
                ))
            });

        repo.product_reader
            .expect_get_product_by_id()
            .returning(|product_id, hub_id| {
                Ok(Some(sample_product(
                    product_id.get(),
                    hub_id.get(),
                    10,
                    450,
                    "USD",
                )))
            });

        repo.order_writer.expect_create_order().times(0);

        let payload = vec![StoreOrderLinePayload {
            product_id: 1,
            quantity: 1,
        }];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn create_store_order_rejects_unknown_product() {
        let mut repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: None,
            public_id: None,
        };

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(|_| Ok((0, vec![sample_price_level(1, true)])));

        repo.product_reader
            .expect_get_product_by_id()
            .returning(|_, _| Ok(None));

        let payload = vec![StoreOrderLinePayload {
            product_id: 1,
            quantity: 1,
        }];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn create_store_order_rejects_missing_price() {
        let mut repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: None,
            public_id: None,
        };

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(|_| Ok((0, vec![sample_price_level(1, true)])));

        repo.product_reader
            .expect_get_product_by_id()
            .returning(|product_id, hub_id| {
                Ok(Some(sample_product(
                    product_id.get(),
                    hub_id.get(),
                    99,
                    500,
                    "USD",
                )))
            });

        let payload = vec![StoreOrderLinePayload {
            product_id: 1,
            quantity: 1,
        }];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn create_store_order_rejects_mixed_currency() {
        let mut repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: None,
            public_id: None,
        };

        repo.price_level_reader
            .expect_list_price_levels()
            .returning(|_| Ok((0, vec![sample_price_level(1, true)])));

        repo.product_reader
            .expect_get_product_by_id()
            .returning(|product_id, hub_id| match product_id.get() {
                1 => Ok(Some(sample_product(1, hub_id.get(), 1, 500, "USD"))),
                2 => Ok(Some(sample_product(2, hub_id.get(), 1, 300, "EUR"))),
                _ => Ok(None),
            });

        let payload = vec![
            StoreOrderLinePayload {
                product_id: 1,
                quantity: 1,
            },
            StoreOrderLinePayload {
                product_id: 2,
                quantity: 1,
            },
        ];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn create_store_order_rejects_invalid_quantities() {
        let repo = MockStoreOrderRepo::new();
        let customer = Customer {
            id: CustomerId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("Customer").unwrap(),
            phone: PhoneNumber::new("+111").unwrap(),
            price_level_id: None,
            public_id: None,
        };

        let payload = vec![StoreOrderLinePayload {
            product_id: 1,
            quantity: 0,
        }];

        let result = create_store_order(1, payload, &customer, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    struct MockListStoreOrdersRepo {
        order_reader: MockOrderReader,
    }

    impl MockListStoreOrdersRepo {
        fn new() -> Self {
            Self {
                order_reader: MockOrderReader::new(),
            }
        }
    }

    impl OrderReader for MockListStoreOrdersRepo {
        fn get_order_by_id(&self, id: OrderId, hub_id: HubId) -> RepositoryResult<Option<Order>> {
            self.order_reader.get_order_by_id(id, hub_id)
        }

        fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<Order>)> {
            self.order_reader.list_orders(query)
        }
    }

    struct MockUpdateStoreOrderRepo {
        order_reader: MockOrderReader,
        order_writer: MockOrderWriter,
    }

    impl MockUpdateStoreOrderRepo {
        fn new() -> Self {
            Self {
                order_reader: MockOrderReader::new(),
                order_writer: MockOrderWriter::new(),
            }
        }
    }

    impl OrderReader for MockUpdateStoreOrderRepo {
        fn get_order_by_id(&self, id: OrderId, hub_id: HubId) -> RepositoryResult<Option<Order>> {
            self.order_reader.get_order_by_id(id, hub_id)
        }

        fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<Order>)> {
            self.order_reader.list_orders(query)
        }
    }

    impl OrderWriter for MockUpdateStoreOrderRepo {
        fn create_order(&self, new_order: &NewOrder) -> RepositoryResult<Order> {
            self.order_writer.create_order(new_order)
        }

        fn update_order(
            &self,
            order_id: OrderId,
            hub_id: HubId,
            updates: &UpdateOrder,
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

    fn sample_order(id: i32, hub_id: i32, customer_id: i32) -> Order {
        Order {
            id: OrderId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            customer_id: Some(CustomerId::new(customer_id).unwrap()),
            reference: None,
            status: DomainOrderStatus::Pending,
            notes: None,
            total_cents: PriceCents::new(500).unwrap(),
            currency: CurrencyCode::new("USD").unwrap(),
            products: vec![OrderProduct::try_new("Item", 500, "USD", 1, None).unwrap()],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        }
    }

    #[test]
    fn update_store_order_applies_request_values() {
        let mut repo = MockUpdateStoreOrderRepo::new();
        let customer = sample_customer();

        let mut existing = sample_order(5, customer.hub_id.get(), customer.id.get());
        existing.shipping_address = Some(OrderShippingAddress::new("Old address").unwrap());
        existing.consignee = Some(OrderConsignee::new("Recipient").unwrap());
        existing.delivery_notes = Some(OrderDeliveryNotes::new("Keep notes").unwrap());
        existing.payer = Some(OrderPayer::new("Old payer").unwrap());

        let order_clone = existing.clone();
        repo.order_reader
            .expect_get_order_by_id()
            .return_once(move |_, _| Ok(Some(order_clone.clone())));

        let mut updated = existing.clone();
        updated.shipping_address = Some(OrderShippingAddress::new("New address").unwrap());
        updated.consignee = None;
        updated.payer = Some(OrderPayer::new("New payer").unwrap());

        let updated_clone = updated.clone();
        repo.order_writer
            .expect_update_order()
            .returning(move |order_id, hub_id, updates| {
                assert_eq!(order_id, updated_clone.id);
                assert_eq!(hub_id, updated_clone.hub_id);
                assert_eq!(
                    updates
                        .shipping_address
                        .as_ref()
                        .map(|value| value.as_str()),
                    Some("New address")
                );
                assert_eq!(updates.consignee, None);
                assert_eq!(
                    updates.delivery_notes.as_ref().map(|value| value.as_str()),
                    Some("Keep notes")
                );
                assert_eq!(
                    updates.payer.as_ref().map(|value| value.as_str()),
                    Some("New payer")
                );
                Ok(updated_clone.clone())
            });

        let values = StoreOrderUpdateValues {
            shipping_address: Some(Some(OrderShippingAddress::new("New address").unwrap())),
            consignee: Some(None),
            delivery_notes: None,
            payer: Some(Some(OrderPayer::new("New payer").unwrap())),
        };

        let response = update_store_order(
            existing.hub_id.get(),
            existing.id.get(),
            values,
            &customer,
            &repo,
        )
        .expect("expected update");

        assert_eq!(response.shipping_address.as_deref(), Some("New address"));
        assert_eq!(response.consignee, None);
        assert_eq!(response.delivery_notes.as_deref(), Some("Keep notes"));
        assert_eq!(response.payer.as_deref(), Some("New payer"));
    }

    #[test]
    fn update_store_order_rejects_unauthorized_customer() {
        let mut repo = MockUpdateStoreOrderRepo::new();
        let customer = sample_customer();
        let order = sample_order(6, customer.hub_id.get(), customer.id.get() + 1);
        let order_id = order.id;

        repo.order_reader
            .expect_get_order_by_id()
            .return_once(move |_, _| Ok(Some(order)));
        repo.order_writer.expect_update_order().never();

        let values = StoreOrderUpdateValues::default();

        let result = update_store_order(
            customer.hub_id.get(),
            order_id.get(),
            values,
            &customer,
            &repo,
        );

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_store_order_rejects_missing_order() {
        let mut repo = MockUpdateStoreOrderRepo::new();
        let customer = sample_customer();

        repo.order_reader
            .expect_get_order_by_id()
            .return_once(|_, _| Ok(None));
        repo.order_writer.expect_update_order().never();

        let values = StoreOrderUpdateValues::default();

        let result = update_store_order(customer.hub_id.get(), 10, values, &customer, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn list_store_orders_returns_orders_for_customer() {
        let mut repo = MockListStoreOrdersRepo::new();
        let customer = sample_customer();
        let match_customer = customer.clone();

        repo.order_reader
            .expect_list_orders()
            .withf(move |query| {
                query.hub_id == match_customer.hub_id
                    && query.customer_id == Some(match_customer.id)
                    && query.pagination.is_none()
            })
            .return_once(move |_| {
                Ok((
                    1,
                    vec![sample_order(
                        1,
                        match_customer.hub_id.get(),
                        match_customer.id.get(),
                    )],
                ))
            });

        let orders = list_store_orders(customer.hub_id.get(), None, &customer, &repo)
            .expect("expected orders");

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].id, 1);
    }

    #[test]
    fn list_store_orders_applies_pagination() {
        let mut repo = MockListStoreOrdersRepo::new();
        let customer = sample_customer();

        repo.order_reader
            .expect_list_orders()
            .withf(|query| {
                query
                    .pagination
                    .as_ref()
                    .is_some_and(|pagination| pagination.page == 2)
            })
            .return_once(|_| Ok((0, Vec::new())));

        let orders = list_store_orders(customer.hub_id.get(), Some(2), &customer, &repo)
            .expect("expected empty orders");

        assert!(orders.is_empty());
    }

    impl CustomerReader for MockOtpRequestRepo {
        fn get_customer_by_id(
            &self,
            id: CustomerId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_phone(
            &self,
            phone: &PhoneNumber,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_phone(phone, hub_id)
        }

        fn list_customers(
            &self,
            query: CustomerListQuery,
        ) -> RepositoryResult<(usize, Vec<Customer>)> {
            self.customer_reader.list_customers(query)
        }
    }

    impl CustomerWriter for MockOtpRequestRepo {
        fn create_customer(
            &self,
            new_customer: &crate::domain::customer::NewCustomer,
        ) -> RepositoryResult<Customer> {
            self.customer_writer.create_customer(new_customer)
        }

        fn assign_price_level_to_customers(
            &self,
            hub_id: HubId,
            customer_ids: &[CustomerId],
            price_level_id: Option<PriceLevelId>,
        ) -> RepositoryResult<()> {
            self.customer_writer.assign_price_level_to_customers(
                hub_id,
                customer_ids,
                price_level_id,
            )
        }

        fn update_customer(
            &self,
            customer_id: CustomerId,
            hub_id: HubId,
            updates: &crate::domain::customer::UpdateCustomer,
        ) -> RepositoryResult<Customer> {
            self.customer_writer
                .update_customer(customer_id, hub_id, updates)
        }
    }

    impl StoreOtpRepository for MockOtpRequestRepo {
        fn get_store_otp(
            &self,
            hub_id: HubId,
            phone: &PhoneNumber,
        ) -> RepositoryResult<Option<DomainStoreOtp>> {
            self.otp_repository.get_store_otp(hub_id, phone)
        }

        fn upsert_store_otp(
            &self,
            new_otp: &DomainNewStoreOtp,
        ) -> RepositoryResult<DomainStoreOtp> {
            self.otp_repository.upsert_store_otp(new_otp)
        }

        fn delete_store_otp(&self, hub_id: HubId, phone: &PhoneNumber) -> RepositoryResult<()> {
            self.otp_repository.delete_store_otp(hub_id, phone)
        }
    }

    fn sample_customer() -> Customer {
        Customer {
            id: CustomerId::new(1).unwrap(),
            hub_id: HubId::new(99).unwrap(),
            name: CustomerName::new("Sample").unwrap(),
            phone: PhoneNumber::new("+15551234").unwrap(),
            price_level_id: None,
            public_id: None,
        }
    }

    #[test]
    fn load_store_session_customer_returns_database_customer() {
        let mut repo = MockCustomerReader::new();
        let expected = sample_customer();
        let match_sample = expected.clone();
        let return_sample = expected.clone();
        repo.expect_get_customer_by_id()
            .withf(move |id, hub_id| {
                id.get() == match_sample.id.get() && hub_id.get() == match_sample.hub_id.get()
            })
            .return_once(move |_, _| Ok(Some(return_sample)));

        let customer = load_store_session_customer(&expected, &repo).expect("expected customer");

        assert_eq!(customer, expected);
    }

    #[test]
    fn load_store_session_customer_rejects_missing_customer() {
        let mut repo = MockCustomerReader::new();
        repo.expect_get_customer_by_id()
            .return_once(|_, _| Ok(None));

        let result = load_store_session_customer(&sample_customer(), &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[derive(Clone)]
    struct TestZmqSender {
        recorded: Arc<Mutex<Vec<ZMQSendSmsMessage>>>,
    }

    impl TestZmqSender {
        fn new() -> Self {
            Self {
                recorded: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn messages(&self) -> Arc<Mutex<Vec<ZMQSendSmsMessage>>> {
            Arc::clone(&self.recorded)
        }
    }

    impl ZmqSenderTrait for TestZmqSender {
        fn send_bytes<'a>(&'a self, bytes: Vec<u8>) -> SendFuture<'a> {
            let recorded = Arc::clone(&self.recorded);
            Box::pin(async move {
                let msg: ZMQSendSmsMessage =
                    serde_json::from_slice(&bytes).map_err(ZmqSenderError::Serialize)?;
                recorded.lock().unwrap().push(msg);
                Ok(())
            })
        }

        fn try_send_bytes(&self, bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
            let msg: ZMQSendSmsMessage =
                serde_json::from_slice(&bytes).map_err(ZmqSenderError::Serialize)?;
            self.recorded.lock().unwrap().push(msg);
            Ok(())
        }

        fn send_multipart<'a>(&'a self, frames: Vec<Vec<u8>>) -> SendFuture<'a> {
            let recorded = Arc::clone(&self.recorded);
            Box::pin(async move {
                let mut iter = frames.into_iter();
                iter.next();
                if let Some(payload) = iter.next() {
                    let msg: ZMQSendSmsMessage =
                        serde_json::from_slice(&payload).map_err(ZmqSenderError::Serialize)?;
                    recorded.lock().unwrap().push(msg);
                    Ok(())
                } else {
                    Err(ZmqSenderError::QueueFull)
                }
            })
        }
    }

    #[derive(Clone)]
    struct TestZmqClientSender {
        recorded: Arc<Mutex<Vec<ZmqClientMessage>>>,
    }

    impl TestZmqClientSender {
        fn new() -> Self {
            Self {
                recorded: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn messages(&self) -> Arc<Mutex<Vec<ZmqClientMessage>>> {
            Arc::clone(&self.recorded)
        }
    }

    impl ZmqSenderTrait for TestZmqClientSender {
        fn send_bytes<'a>(&'a self, bytes: Vec<u8>) -> SendFuture<'a> {
            let recorded = Arc::clone(&self.recorded);
            Box::pin(async move {
                let msg: ZmqClientMessage =
                    serde_json::from_slice(&bytes).map_err(ZmqSenderError::Serialize)?;
                recorded.lock().unwrap().push(msg);
                Ok(())
            })
        }

        fn try_send_bytes(&self, bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
            let msg: ZmqClientMessage =
                serde_json::from_slice(&bytes).map_err(ZmqSenderError::Serialize)?;
            self.recorded.lock().unwrap().push(msg);
            Ok(())
        }

        fn send_multipart<'a>(&'a self, frames: Vec<Vec<u8>>) -> SendFuture<'a> {
            let recorded = Arc::clone(&self.recorded);
            Box::pin(async move {
                let mut iter = frames.into_iter();
                iter.next();
                if let Some(payload) = iter.next() {
                    let msg: ZmqClientMessage =
                        serde_json::from_slice(&payload).map_err(ZmqSenderError::Serialize)?;
                    recorded.lock().unwrap().push(msg);
                    Ok(())
                } else {
                    Err(ZmqSenderError::QueueFull)
                }
            })
        }
    }

    #[actix_web::rt::test]
    async fn request_store_otp_accepts_first_request() {
        let phone = "+15551234".to_string();
        let payload = StoreOtpRequestPayload {
            phone: phone.clone(),
        };

        let mut repo = MockStoreOtpRepository::new();
        repo.expect_get_store_otp()
            .withf(|hub_id, phone| hub_id.get() == 99 && phone.as_str() == "+15551234")
            .return_once(|_, _| Ok(None));
        repo.expect_upsert_store_otp()
            .withf(|new_otp| {
                new_otp.code.as_str().len() == 6
                    && new_otp.code.as_str().chars().all(|ch| ch.is_ascii_digit())
            })
            .return_once(|new_otp| {
                Ok(DomainStoreOtp {
                    hub_id: new_otp.hub_id,
                    phone: new_otp.phone.clone(),
                    code: new_otp.code.clone(),
                    expires_at: new_otp.expires_at,
                    last_sent_at: new_otp.last_sent_at,
                })
            });

        let zmq_sender = TestZmqSender::new();
        let sms_sender = "test-sms-sender".to_string();

        let response = request_store_otp(99, payload, &repo, &zmq_sender, &sms_sender)
            .await
            .expect("expected success");

        let recorded = zmq_sender.messages();
        let guard = recorded.lock().unwrap();
        assert_eq!(guard.len(), 1);
        let message = &guard[0];
        assert_eq!(message.sender_id, sms_sender);
        assert_eq!(message.phone_number, phone);
        assert!(message.message.starts_with("Your OTP is "));

        assert!(response.success);
    }

    #[actix_web::rt::test]
    async fn request_store_otp_throttles_recent_requests() {
        let payload = StoreOtpRequestPayload {
            phone: "+15551234".to_string(),
        };

        let mut repo = MockStoreOtpRepository::new();
        repo.expect_get_store_otp().return_once(|_, _| {
            Ok(Some(DomainStoreOtp {
                hub_id: HubId::new(99).unwrap(),
                phone: phone_number("+15551234"),
                code: otp_code("123456"),
                expires_at: Utc::now().naive_utc() + Duration::minutes(OTP_EXPIRY_MINUTES),
                last_sent_at: Utc::now().naive_utc(),
            }))
        });
        repo.expect_upsert_store_otp().never();

        let zmq_sender = TestZmqSender::new();
        let sms_sender = "test-sms-sender".to_string();

        let result = request_store_otp(99, payload, &repo, &zmq_sender, &sms_sender).await;

        match result {
            Err(ServiceError::Form(message)) => assert_eq!(message, OTP_THROTTLE_MESSAGE),
            other => panic!("unexpected result: {:?}", other),
        }

        assert!(zmq_sender.messages().lock().unwrap().is_empty());
    }

    #[actix_web::rt::test]
    async fn verify_store_otp_checks_code_and_expiry() {
        let payload = StoreOtpVerifyPayload {
            phone: "+15551234".to_string(),
            otp: "123456".to_string(),
        };

        let mut repo = MockOtpRequestRepo::new();
        repo.otp_repository
            .expect_get_store_otp()
            .return_once(|_, _| {
                Ok(Some(DomainStoreOtp {
                    hub_id: HubId::new(1).unwrap(),
                    phone: phone_number("+15551234"),
                    code: otp_code("123456"),
                    expires_at: Utc::now().naive_utc() + Duration::minutes(OTP_EXPIRY_MINUTES),
                    last_sent_at: Utc::now().naive_utc(),
                }))
            });
        repo.otp_repository
            .expect_delete_store_otp()
            .withf(|hub_id, phone| hub_id.get() == 1 && phone.as_str() == "+15551234")
            .return_once(|_, _| Ok(()));
        repo.otp_repository.expect_upsert_store_otp().never();
        repo.customer_reader
            .expect_get_customer_by_phone()
            .return_once(|_, _| Ok(Some(sample_customer())));
        repo.customer_writer.expect_create_customer().never();

        let zmq_sender = TestZmqClientSender::new();

        let response = verify_store_otp(1, payload, &repo, &zmq_sender)
            .await
            .expect("expected success");

        assert!(!zmq_sender.messages().lock().unwrap().is_empty());

        assert!(response.success);
        assert_eq!(response.customer, sample_customer());
    }

    #[actix_web::rt::test]
    async fn verify_store_otp_creates_customer_when_missing() {
        let payload = StoreOtpVerifyPayload {
            phone: "+15551234".to_string(),
            otp: "123456".to_string(),
        };
        let created_customer = Customer {
            id: CustomerId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CustomerName::new("+15551234").unwrap(),
            phone: PhoneNumber::new("+15551234").unwrap(),
            price_level_id: None,
            public_id: None,
        };

        let mut repo = MockOtpRequestRepo::new();
        repo.otp_repository
            .expect_get_store_otp()
            .return_once(|_, _| {
                Ok(Some(DomainStoreOtp {
                    hub_id: HubId::new(1).unwrap(),
                    phone: phone_number("+15551234"),
                    code: otp_code("123456"),
                    expires_at: Utc::now().naive_utc() + Duration::minutes(OTP_EXPIRY_MINUTES),
                    last_sent_at: Utc::now().naive_utc(),
                }))
            });
        repo.otp_repository
            .expect_delete_store_otp()
            .withf(|hub_id, phone| hub_id.get() == 1 && phone.as_str() == "+15551234")
            .return_once(|_, _| Ok(()));
        repo.otp_repository.expect_upsert_store_otp().never();
        repo.customer_reader
            .expect_get_customer_by_phone()
            .return_once(|_, _| Ok(None));
        repo.customer_writer
            .expect_create_customer()
            .withf(|new_customer| {
                new_customer.hub_id.get() == 1
                    && new_customer.phone.as_str() == "+15551234"
                    && new_customer.name.as_str() == "+15551234"
            })
            .return_once({
                let created_customer = created_customer.clone();
                move |_| Ok(created_customer)
            });

        let zmq_sender = TestZmqClientSender::new();

        let response = verify_store_otp(1, payload, &repo, &zmq_sender)
            .await
            .expect("expected success");

        let recorded = zmq_sender.messages();
        let guard = recorded.lock().unwrap();
        assert_eq!(guard.len(), 1);
        let message = &guard[0];
        assert_eq!(message.hub_id, 1);
        assert_eq!(message.name, "+15551234");
        assert_eq!(message.email.as_deref(), None);
        assert_eq!(message.phone.as_deref(), Some("+15551234"));

        assert!(response.success);
        assert_eq!(response.customer, created_customer);
    }

    #[actix_web::rt::test]
    async fn verify_store_otp_rejects_invalid_code() {
        let payload = StoreOtpVerifyPayload {
            phone: "+15551234".to_string(),
            otp: "000000".to_string(),
        };

        let mut repo = MockOtpRequestRepo::new();
        repo.otp_repository
            .expect_get_store_otp()
            .return_once(|_, _| {
                Ok(Some(DomainStoreOtp {
                    hub_id: HubId::new(1).unwrap(),
                    phone: phone_number("+15551234"),
                    code: otp_code("123456"),
                    expires_at: Utc::now().naive_utc() + Duration::minutes(OTP_EXPIRY_MINUTES),
                    last_sent_at: Utc::now().naive_utc(),
                }))
            });
        repo.otp_repository.expect_delete_store_otp().never();
        repo.otp_repository.expect_upsert_store_otp().never();
        repo.customer_reader.expect_get_customer_by_phone().never();
        repo.customer_writer.expect_create_customer().never();
        repo.customer_reader.expect_get_customer_by_phone().never();
        repo.customer_writer.expect_create_customer().never();

        let zmq_sender = TestZmqClientSender::new();

        let result = verify_store_otp(1, payload, &repo, &zmq_sender).await;

        match result {
            Err(ServiceError::Form(message)) => assert_eq!(message, OTP_INVALID_MESSAGE),
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[actix_web::rt::test]
    async fn verify_store_otp_rejects_expired_code() {
        let payload = StoreOtpVerifyPayload {
            phone: "+15551234".to_string(),
            otp: "123456".to_string(),
        };

        let mut repo = MockOtpRequestRepo::new();
        repo.otp_repository
            .expect_get_store_otp()
            .return_once(|_, _| {
                Ok(Some(DomainStoreOtp {
                    hub_id: HubId::new(1).unwrap(),
                    phone: phone_number("+15551234"),
                    code: otp_code("123456"),
                    expires_at: Utc::now().naive_utc() - Duration::minutes(1),
                    last_sent_at: Utc::now().naive_utc(),
                }))
            });
        repo.otp_repository.expect_delete_store_otp().never();
        repo.otp_repository.expect_upsert_store_otp().never();

        let zmq_sender = TestZmqClientSender::new();

        let result = verify_store_otp(1, payload, &repo, &zmq_sender).await;

        match result {
            Err(ServiceError::Form(message)) => assert_eq!(message, OTP_INVALID_MESSAGE),
            other => panic!("unexpected result: {:?}", other),
        }
    }

    struct MockStoreProductRepo {
        product_reader: MockProductReader,
        price_level_reader: MockPriceLevelReader,
    }

    impl MockStoreProductRepo {
        fn new(
            product_reader: MockProductReader,
            price_level_reader: MockPriceLevelReader,
        ) -> Self {
            Self {
                product_reader,
                price_level_reader,
            }
        }
    }

    impl ProductReader for MockStoreProductRepo {
        fn get_product_by_id(
            &self,
            id: ProductId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Product>> {
            self.product_reader.get_product_by_id(id, hub_id)
        }

        fn list_products(
            &self,
            query: ProductListQuery,
        ) -> RepositoryResult<(usize, Vec<Product>)> {
            self.product_reader.list_products(query)
        }
    }

    impl PriceLevelReader for MockStoreProductRepo {
        fn get_price_level_by_id(
            &self,
            id: PriceLevelId,
            hub_id: HubId,
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
    fn load_categories_filters_archived_items() {
        let mut repo = MockCategoryReader::new();
        let categories = vec![
            Category {
                id: CategoryId::new(1).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                parent_id: None,
                name: CategoryName::new("Coffee").unwrap(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
            Category {
                id: CategoryId::new(2).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                parent_id: None,
                name: CategoryName::new("Archived").unwrap(),
                description: None,
                is_archived: true,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
        ];

        repo.expect_list_categories()
            .withf(|query| query.hub_id.get() == 1 && !query.include_archived)
            .return_once(move |_| Ok((2, categories.clone())));

        let result = load_store_categories(1, StoreCategoryFilters::default(), &repo)
            .expect("load categories");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "Coffee");
    }

    #[test]
    fn load_categories_filters_by_parent_id() {
        let mut repo = MockCategoryReader::new();
        let categories = vec![
            Category {
                id: CategoryId::new(1).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                parent_id: None,
                name: CategoryName::new("Root").unwrap(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
            Category {
                id: CategoryId::new(2).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                parent_id: Some(CategoryId::new(1).unwrap()),
                name: CategoryName::new("Child").unwrap(),
                description: None,
                is_archived: false,
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_url: None,
            },
        ];

        let categories_clone = categories.clone();
        repo.expect_list_categories()
            .withf(|query| query.hub_id.get() == 1 && !query.include_archived)
            .times(2)
            .returning(move |_| Ok((2, categories_clone.clone())));

        let roots = load_store_categories(1, StoreCategoryFilters::default(), &repo)
            .expect("load root categories");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, 1);

        let children = load_store_categories(1, StoreCategoryFilters { parent_id: Some(1) }, &repo)
            .expect("load child categories");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, 2);
    }

    #[test]
    fn load_products_includes_tags() {
        let mut product_reader = MockProductReader::new();
        let products = vec![
            Product {
                id: ProductId::new(1).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: ProductName::new("Coffee").unwrap(),
                sku: Some(ProductSku::new("SKU-1").unwrap()),
                description: Some(ProductDescription::new("Fresh beans").unwrap()),
                units: Some(ProductUnits::new("kg").unwrap()),
                currency: CurrencyCode::new("USD").unwrap(),
                is_archived: false,
                category_id: Some(CategoryId::new(1).unwrap()),
                price_levels: vec![ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(1).unwrap(),
                    product_id: ProductId::new(1).unwrap(),
                    price_level_id: PriceLevelId::new(10).unwrap(),
                    price_cents: PriceCents::new(1_299).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                }],
                tags: vec![tag(1, 1, "Organic")],
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_urls: vec![ImageUrl::new("https://example.com/coffee.png").unwrap()],
                amount: None,
            },
            Product {
                id: ProductId::new(2).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: ProductName::new("Retired").unwrap(),
                sku: None,
                description: None,
                units: None,
                currency: CurrencyCode::new("USD").unwrap(),
                is_archived: true,
                category_id: None,
                price_levels: Vec::new(),
                tags: Vec::new(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                image_urls: Vec::new(),
                amount: None,
            },
        ];

        let products_clone = products.clone();
        product_reader
            .expect_list_products()
            .withf(|query| {
                query.hub_id == HubId::new(1).unwrap()
                    && !query.include_archived
                    && query.only_without_category
                    && query.category_id.is_none()
            })
            .return_once(move |_| Ok((2, products_clone)));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![PriceLevel {
            id: PriceLevelId::new(10).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: PriceLevelName::new("Default").unwrap(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            is_default: true,
        }];

        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(move |_| Ok((1, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let result = load_store_products(1, StoreProductFilters::default(), None, &repo)
            .expect("load products");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "Coffee");
        assert_eq!(result[0].tags.len(), 1);
        assert_eq!(result[0].tags[0].name, "Organic");
        assert_eq!(result[0].price_cents, Some(1_299));
        assert_eq!(
            result[0].image_urls,
            vec!["https://example.com/coffee.png".to_string()]
        );
    }

    #[test]
    fn load_store_products_uses_customer_price_level_when_present() {
        let mut product_reader = MockProductReader::new();
        let products = vec![Product {
            id: ProductId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Coffee").unwrap(),
            sku: Some(ProductSku::new("SKU-1").unwrap()),
            description: Some(ProductDescription::new("Fresh beans").unwrap()),
            units: Some(ProductUnits::new("kg").unwrap()),
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: Some(CategoryId::new(1).unwrap()),
            price_levels: vec![
                ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(1).unwrap(),
                    product_id: ProductId::new(1).unwrap(),
                    price_level_id: PriceLevelId::new(10).unwrap(),
                    price_cents: PriceCents::new(450).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                },
                ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(2).unwrap(),
                    product_id: ProductId::new(1).unwrap(),
                    price_level_id: PriceLevelId::new(11).unwrap(),
                    price_cents: PriceCents::new(500).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                },
            ],
            tags: vec![tag(1, 1, "Organic")],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: vec![ImageUrl::new("https://example.com/coffee.png").unwrap()],
            amount: None,
        }];

        let product_clone = products.clone();
        let expected_price_level_id = PriceLevelId::new(11).unwrap();
        product_reader
            .expect_list_products()
            .withf(move |query| {
                query.hub_id == HubId::new(1).unwrap()
                    && !query.include_archived
                    && query.only_without_category
                    && query.category_id.is_none()
                    && query.price_level_id == Some(expected_price_level_id)
            })
            .return_once(move |_| Ok((1, product_clone)));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![
            PriceLevel {
                id: PriceLevelId::new(10).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Default").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: true,
            },
            PriceLevel {
                id: PriceLevelId::new(11).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Premium").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: false,
            },
        ];

        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(move |_| Ok((2, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let mut customer = sample_customer();
        customer.hub_id = HubId::new(1).unwrap();
        customer.price_level_id = Some(PriceLevelId::new(11).unwrap());

        let result = load_store_products(1, StoreProductFilters::default(), Some(&customer), &repo)
            .expect("load products for authenticated customer");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].price_cents, Some(500));
        assert_eq!(result[0].base_price_cents, Some(450));
    }

    #[test]
    fn load_store_products_hides_products_without_customer_price_level_rate() {
        let mut product_reader = MockProductReader::new();
        let products = vec![Product {
            id: ProductId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Coffee").unwrap(),
            sku: Some(ProductSku::new("SKU-1").unwrap()),
            description: Some(ProductDescription::new("Fresh beans").unwrap()),
            units: Some(ProductUnits::new("kg").unwrap()),
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: Some(CategoryId::new(1).unwrap()),
            price_levels: vec![ProductPriceLevelRate {
                id: ProductPriceLevelRateId::new(1).unwrap(),
                product_id: ProductId::new(1).unwrap(),
                price_level_id: PriceLevelId::new(10).unwrap(),
                price_cents: PriceCents::new(450).unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
            }],
            tags: vec![],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: Vec::new(),
            amount: None,
        }];

        let product_clone = products.clone();
        let expected_price_level_id = PriceLevelId::new(11).unwrap();
        product_reader
            .expect_list_products()
            .withf(move |query| {
                query.hub_id == HubId::new(1).unwrap()
                    && !query.include_archived
                    && query.only_without_category
                    && query.category_id.is_none()
                    && query.price_level_id == Some(expected_price_level_id)
            })
            .return_once(move |_| Ok((1, product_clone)));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![
            PriceLevel {
                id: PriceLevelId::new(10).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Default").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: true,
            },
            PriceLevel {
                id: PriceLevelId::new(11).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Premium").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: false,
            },
        ];

        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(move |_| Ok((2, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let mut customer = sample_customer();
        customer.hub_id = HubId::new(1).unwrap();
        customer.price_level_id = Some(PriceLevelId::new(11).unwrap());

        let result = load_store_products(1, StoreProductFilters::default(), Some(&customer), &repo)
            .expect("load products for authenticated customer");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn load_store_product_fetches_active_product() {
        let mut product_reader = MockProductReader::new();
        let product = Product {
            id: ProductId::new(7).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Latte").unwrap(),
            sku: Some(ProductSku::new("SKU-LATTE").unwrap()),
            description: Some(ProductDescription::new("Steamed milk with espresso").unwrap()),
            units: Some(ProductUnits::new("cup").unwrap()),
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: Some(CategoryId::new(3).unwrap()),
            price_levels: vec![
                ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(1).unwrap(),
                    product_id: ProductId::new(7).unwrap(),
                    price_level_id: PriceLevelId::new(10).unwrap(),
                    price_cents: PriceCents::new(450).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                },
                ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(2).unwrap(),
                    product_id: ProductId::new(7).unwrap(),
                    price_level_id: PriceLevelId::new(11).unwrap(),
                    price_cents: PriceCents::new(500).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                },
            ],
            tags: vec![tag(2, 1, "Barista's choice")],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: vec![ImageUrl::new("https://example.com/latte.png").unwrap()],
            amount: None,
        };

        product_reader
            .expect_get_product_by_id()
            .withf(|id, hub_id| id.get() == 7 && hub_id.get() == 1)
            .return_once(move |_, _| Ok(Some(product.clone())));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![
            PriceLevel {
                id: PriceLevelId::new(10).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Default").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: true,
            },
            PriceLevel {
                id: PriceLevelId::new(11).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Premium").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: false,
            },
        ];
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(move |_| Ok((2, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let result = load_store_product(1, 7, None, &repo).expect("load single product");
        let product = result.expect("product should be present");
        assert_eq!(product.id, 7);
        assert_eq!(product.price_cents, Some(450));
        assert_eq!(product.tags.len(), 1);
        assert_eq!(product.tags[0].name, "Barista's choice");
        assert_eq!(
            product.image_urls,
            vec!["https://example.com/latte.png".to_string()]
        );
    }

    #[test]
    fn load_store_product_uses_customer_price_level() {
        let mut product_reader = MockProductReader::new();
        let product = Product {
            id: ProductId::new(7).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Latte").unwrap(),
            sku: Some(ProductSku::new("SKU-LATTE").unwrap()),
            description: Some(ProductDescription::new("Steamed milk with espresso").unwrap()),
            units: Some(ProductUnits::new("cup").unwrap()),
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: Some(CategoryId::new(3).unwrap()),
            price_levels: vec![
                ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(1).unwrap(),
                    product_id: ProductId::new(7).unwrap(),
                    price_level_id: PriceLevelId::new(10).unwrap(),
                    price_cents: PriceCents::new(450).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                },
                ProductPriceLevelRate {
                    id: ProductPriceLevelRateId::new(2).unwrap(),
                    product_id: ProductId::new(7).unwrap(),
                    price_level_id: PriceLevelId::new(11).unwrap(),
                    price_cents: PriceCents::new(500).unwrap(),
                    created_at: sample_timestamp(),
                    updated_at: sample_timestamp(),
                },
            ],
            tags: vec![tag(2, 1, "Barista's choice")],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: vec![ImageUrl::new("https://example.com/latte.png").unwrap()],
            amount: None,
        };

        product_reader
            .expect_get_product_by_id()
            .withf(|id, hub_id| id.get() == 7 && hub_id.get() == 1)
            .return_once(move |_, _| Ok(Some(product.clone())));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![
            PriceLevel {
                id: PriceLevelId::new(10).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Default").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: true,
            },
            PriceLevel {
                id: PriceLevelId::new(11).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Premium").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: false,
            },
        ];
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(move |_| Ok((2, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let mut customer = sample_customer();
        customer.hub_id = HubId::new(1).unwrap();
        customer.price_level_id = Some(PriceLevelId::new(11).unwrap());

        let result = load_store_product(1, 7, Some(&customer), &repo).expect("load single product");
        let product = result.expect("product should be present");
        assert_eq!(product.price_cents, Some(500));
        assert_eq!(product.base_price_cents, Some(450));
    }

    #[test]
    fn load_store_product_hides_product_without_customer_price_level_rate() {
        let mut product_reader = MockProductReader::new();
        let product = Product {
            id: ProductId::new(7).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Latte").unwrap(),
            sku: Some(ProductSku::new("SKU-LATTE").unwrap()),
            description: Some(ProductDescription::new("Steamed milk with espresso").unwrap()),
            units: Some(ProductUnits::new("cup").unwrap()),
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: Some(CategoryId::new(3).unwrap()),
            price_levels: vec![ProductPriceLevelRate {
                id: ProductPriceLevelRateId::new(1).unwrap(),
                product_id: ProductId::new(7).unwrap(),
                price_level_id: PriceLevelId::new(10).unwrap(),
                price_cents: PriceCents::new(450).unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
            }],
            tags: vec![],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: Vec::new(),
            amount: None,
        };

        product_reader
            .expect_get_product_by_id()
            .withf(|id, hub_id| id.get() == 7 && hub_id.get() == 1)
            .return_once(move |_, _| Ok(Some(product.clone())));

        let mut price_level_reader = MockPriceLevelReader::new();
        let price_levels = vec![
            PriceLevel {
                id: PriceLevelId::new(10).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Default").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: true,
            },
            PriceLevel {
                id: PriceLevelId::new(11).unwrap(),
                hub_id: HubId::new(1).unwrap(),
                name: PriceLevelName::new("Premium").unwrap(),
                created_at: sample_timestamp(),
                updated_at: sample_timestamp(),
                is_default: false,
            },
        ];
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(move |_| Ok((2, price_levels.clone())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let mut customer = sample_customer();
        customer.hub_id = HubId::new(1).unwrap();
        customer.price_level_id = Some(PriceLevelId::new(11).unwrap());

        let result = load_store_product(1, 7, Some(&customer), &repo).expect("load single product");
        assert!(result.is_none());
    }

    #[test]
    fn load_store_product_returns_none_for_missing_or_archived() {
        let mut product_reader = MockProductReader::new();
        let archived_product = Product {
            id: ProductId::new(9).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Retired").unwrap(),
            sku: None,
            description: None,
            units: None,
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: true,
            category_id: None,
            price_levels: Vec::new(),
            tags: Vec::new(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: Vec::new(),
            amount: None,
        };

        product_reader
            .expect_get_product_by_id()
            .withf(|id, hub_id| id.get() == 8 && hub_id.get() == 1)
            .return_once(|_, _| Ok(None));
        product_reader
            .expect_get_product_by_id()
            .withf(|id, hub_id| id.get() == 9 && hub_id.get() == 1)
            .return_once(move |_, _| Ok(Some(archived_product.clone())));

        let price_level_reader = MockPriceLevelReader::new();
        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let missing = load_store_product(1, 8, None, &repo).expect("load missing product");
        assert!(missing.is_none());

        let archived = load_store_product(1, 9, None, &repo).expect("load archived product");
        assert!(archived.is_none());
    }

    #[test]
    fn load_store_products_defaults_to_uncategorized() {
        let mut product_reader = MockProductReader::new();
        let uncategorized = Product {
            id: ProductId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: ProductName::new("Andromeda").unwrap(),
            sku: None,
            description: None,
            units: None,
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: None,
            price_levels: Vec::new(),
            tags: Vec::new(),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            image_urls: Vec::new(),
            amount: None,
        };

        product_reader
            .expect_list_products()
            .withf(|query| {
                query.hub_id == HubId::new(1).unwrap()
                    && query.category_id.is_none()
                    && query.only_without_category
                    && !query.include_archived
            })
            .return_once(move |_| Ok((1, vec![uncategorized.clone()])));

        let mut price_level_reader = MockPriceLevelReader::new();
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(|_| Ok((0, Vec::new())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let result = load_store_products(1, StoreProductFilters::default(), None, &repo)
            .expect("load products");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].category_id, None);
        assert_eq!(result[0].price_cents, None);
    }

    #[test]
    fn load_store_products_applies_filters() {
        let mut product_reader = MockProductReader::new();

        product_reader
            .expect_list_products()
            .withf(|query| {
                query.hub_id == HubId::new(1).unwrap()
                    && query.category_id == Some(CategoryId::new(3).unwrap())
                    && !query.only_without_category
                    && query.search.as_deref() == Some("coffee")
                    && matches!(
                        query.pagination.as_ref(),
                        Some(pagination)
                            if pagination.page == 2
                                && pagination.per_page == DEFAULT_ITEMS_PER_PAGE
                    )
            })
            .return_once(|_| Ok((0, Vec::new())));

        let mut price_level_reader = MockPriceLevelReader::new();
        price_level_reader
            .expect_list_price_levels()
            .withf(|query| query.hub_id.get() == 1)
            .return_once(|_| Ok((0, Vec::new())));

        let repo = MockStoreProductRepo::new(product_reader, price_level_reader);

        let filters = StoreProductFilters {
            category_id: Some(3),
            search: Some(" coffee ".to_string()),
            page: Some(2),
            tag_id: None,
        };

        let result = load_store_products(1, filters, None, &repo).expect("load products");
        assert!(result.is_empty());
    }
}
