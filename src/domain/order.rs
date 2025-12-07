//! Order domain models with product snapshots and status lifecycle.

use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    CurrencyCode, CustomerId, HubId, OrderConsignee, OrderDeliveryNotes, OrderId, OrderNotes,
    OrderPayer, OrderReference, OrderShippingAddress, PriceCents, ProductDescription, ProductId,
    ProductName, ProductQuantity, ProductSku, TypeConstraintError,
};

/// Possible lifecycle states for an order managed by a hub.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderStatus {
    /// Order has been created but not yet submitted for processing.
    #[default]
    Draft,
    /// Order has been submitted and awaits processing.
    Pending,
    /// Order is currently being fulfilled.
    Processing,
    /// Order has been fulfilled and is considered complete.
    Completed,
    /// Order has been cancelled and should not be processed further.
    Cancelled,
}

impl TryFrom<&str> for OrderStatus {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Draft" => Ok(Self::Draft),
            "Pending" => Ok(Self::Pending),
            "Processing" => Ok(Self::Processing),
            "Completed" => Ok(Self::Completed),
            "Cancelled" => Ok(Self::Cancelled),
            _ => Err(TypeConstraintError::InvalidOrderStatus),
        }
    }
}

impl From<OrderStatus> for &'static str {
    fn from(value: OrderStatus) -> Self {
        match value {
            OrderStatus::Draft => "Draft",
            OrderStatus::Pending => "Pending",
            OrderStatus::Processing => "Processing",
            OrderStatus::Completed => "Completed",
            OrderStatus::Cancelled => "Cancelled",
        }
    }
}

impl From<OrderStatus> for String {
    fn from(value: OrderStatus) -> Self {
        <&'static str>::from(value).to_owned()
    }
}

/// Domain representation of an order belonging to a hub.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    /// Unique identifier of the order.
    pub id: OrderId,
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional reference to the customer placing the order.
    pub customer_id: Option<CustomerId>,
    /// External human-friendly reference for the order.
    pub reference: Option<OrderReference>,
    /// Current lifecycle status of the order.
    pub status: OrderStatus,
    /// Optional notes supplied by the operator.
    pub notes: Option<OrderNotes>,
    /// Total amount represented in the smallest currency unit (for example cents).
    pub total_cents: PriceCents,
    /// ISO 4217 currency code used for the order total.
    pub currency: CurrencyCode,
    /// Product snapshots captured when the order was created.
    pub products: Vec<OrderProduct>,
    /// Timestamp for when the order record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the order record.
    pub updated_at: NaiveDateTime,
    /// Optional shipping address associated with the order.
    pub shipping_address: Option<OrderShippingAddress>,
    /// Optional consignee information associated with the order.
    pub consignee: Option<OrderConsignee>,
    /// Optional delivery notes associated with the order.
    pub delivery_notes: Option<OrderDeliveryNotes>,
    /// Optional payer information associated with the order.
    pub payer: Option<OrderPayer>,
}

/// Payload required to insert a new order for a hub.
#[derive(Debug, Clone)]
pub struct NewOrder {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional reference to the customer placing the order.
    pub customer_id: Option<CustomerId>,
    /// External human-friendly reference for the order.
    pub reference: Option<OrderReference>,
    /// Optional notes supplied by the operator.
    pub notes: Option<OrderNotes>,
    /// Total amount represented in the smallest currency unit (for example cents).
    pub total_cents: PriceCents,
    /// ISO 4217 currency code used for the order total.
    pub currency: CurrencyCode,
    /// Product snapshots captured when the order was created.
    pub products: Vec<OrderProduct>,
    /// Current lifecycle status of the order.
    pub status: OrderStatus,
}

/// Static snapshot of a product that was added to an order.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderProduct {
    /// Identifier of the original product, if it still exists.
    pub product_id: Option<ProductId>,
    /// Human-readable name captured at the time of ordering.
    pub name: ProductName,
    /// Stock keeping unit captured at the time of ordering.
    pub sku: Option<ProductSku>,
    /// Description captured at the time of ordering.
    pub description: Option<ProductDescription>,
    /// Price represented in the smallest currency unit for the ordered quantity.
    pub price_cents: PriceCents,
    /// ISO 4217 currency captured at the time of ordering.
    pub currency: CurrencyCode,
    /// Quantity of the product ordered.
    pub quantity: ProductQuantity,
    /// Default price represented in the smallest currency unit.
    pub default_price_cents: Option<PriceCents>,
}

impl OrderProduct {
    /// Create a new ordered product snapshot using the supplied fields.
    pub fn new(
        name: ProductName,
        price_cents: PriceCents,
        currency: CurrencyCode,
        quantity: ProductQuantity,
        default_price_cents: Option<PriceCents>,
    ) -> Self {
        Self {
            product_id: None,
            name,
            sku: None,
            description: None,
            price_cents,
            currency,
            quantity,
            default_price_cents,
        }
    }

    /// Attempt to create a new ordered product snapshot from raw inputs.
    pub fn try_new(
        name: impl Into<String>,
        price_cents: i32,
        currency: impl Into<String>,
        quantity: i32,
        default_price_cents: Option<i32>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ProductName::new(name)?,
            PriceCents::new(price_cents)?,
            CurrencyCode::new(currency)?,
            ProductQuantity::new(quantity)?,
            default_price_cents.map(PriceCents::new).transpose()?,
        ))
    }

    /// Associate the snapshot with the current product identifier.
    pub fn with_product_id(mut self, product_id: ProductId) -> Self {
        self.product_id = Some(product_id);
        self
    }

    /// Capture the SKU value alongside the snapshot.
    pub fn with_sku(mut self, sku: ProductSku) -> Self {
        self.sku = Some(sku);
        self
    }

    /// Capture the description value alongside the snapshot.
    pub fn with_description(mut self, description: ProductDescription) -> Self {
        self.description = Some(description);
        self
    }
}

impl NewOrder {
    /// Build a new order payload with the supplied details and no initial products.
    pub fn new(hub_id: HubId, total_cents: PriceCents, currency: CurrencyCode) -> Self {
        Self {
            hub_id,
            customer_id: None,
            reference: None,
            notes: None,
            total_cents,
            currency,
            status: OrderStatus::default(),
            products: Vec::new(),
        }
    }

    /// Attempt to build a new order from raw inputs.
    pub fn try_new(
        hub_id: i32,
        total_cents: i32,
        currency: impl Into<String>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            PriceCents::new(total_cents)?,
            CurrencyCode::new(currency)?,
        ))
    }

    /// Attach a customer identifier to the order payload.
    pub fn with_customer_id(mut self, customer_id: CustomerId) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    /// Attach an external reference identifier to the order payload.
    pub fn with_reference(mut self, reference: OrderReference) -> Self {
        self.reference = Some(reference);
        self
    }

    /// Attach operator notes to the order payload.
    pub fn with_notes(mut self, notes: OrderNotes) -> Self {
        self.notes = Some(notes);
        self
    }

    /// Override the default status for the new order.
    pub fn with_status(mut self, status: OrderStatus) -> Self {
        self.status = status;
        self
    }

    /// Attach product snapshots to the order payload.
    pub fn with_products(mut self, products: impl Into<Vec<OrderProduct>>) -> Self {
        self.products = products.into();
        self
    }
}

/// Patch data applied when updating an existing order.
#[derive(Debug, Clone)]
pub struct UpdateOrder {
    /// Status update.
    pub status: OrderStatus,
    /// Optional notes update.
    pub notes: Option<OrderNotes>,
    /// Optional external reference update.
    pub reference: Option<OrderReference>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
    /// Optional shipping address.
    pub shipping_address: Option<OrderShippingAddress>,
    /// Optional consignee.
    pub consignee: Option<OrderConsignee>,
    /// Optional delivery notes.
    pub delivery_notes: Option<OrderDeliveryNotes>,
    /// Optional payer.
    pub payer: Option<OrderPayer>,
}

/// Query definition used to list orders for a hub.
#[derive(Debug, Clone)]
pub struct OrderListQuery {
    /// Owning hub identifier.
    pub hub_id: HubId,
    /// Optional status filter.
    pub status: Option<OrderStatus>,
    /// Optional customer identifier filter.
    pub customer_id: Option<CustomerId>,
    /// Optional search term that matches the reference or notes.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl OrderListQuery {
    /// Construct a query that targets all orders belonging to `hub_id`.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            status: None,
            customer_id: None,
            search: None,
            pagination: None,
        }
    }

    /// Attempt to build from raw hub identifier.
    pub fn try_new(hub_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(HubId::new(hub_id)?))
    }

    /// Filter the results by the provided status.
    pub fn status(mut self, status: OrderStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter the results by customer identifier.
    pub fn customer_id(mut self, customer_id: CustomerId) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    /// Filter the results by a search term applied to notes or reference fields.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Apply pagination to the query with the given page number and page size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}
