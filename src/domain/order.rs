use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

/// Possible lifecycle states for an order managed by a hub.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    /// Order has been created but not yet submitted for processing.
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

impl Default for OrderStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl From<&str> for OrderStatus {
    fn from(value: &str) -> Self {
        match value {
            "Draft" => Self::Draft,
            "Pending" => Self::Pending,
            "Processing" => Self::Processing,
            "Completed" => Self::Completed,
            "Cancelled" => Self::Cancelled,
            _ => Self::Draft,
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
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional reference to the customer placing the order.
    pub customer_id: Option<i32>,
    /// External human-friendly reference for the order.
    pub reference: Option<String>,
    /// Current lifecycle status of the order.
    pub status: OrderStatus,
    /// Optional notes supplied by the operator.
    pub notes: Option<String>,
    /// Total amount represented in the smallest currency unit (for example cents).
    pub total_cents: i32,
    /// ISO 4217 currency code used for the order total.
    pub currency: String,
    /// Product snapshots captured when the order was created.
    pub products: Vec<OrderProduct>,
    /// Timestamp for when the order record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the order record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new order for a hub.
#[derive(Debug, Clone)]
pub struct NewOrder {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional reference to the customer placing the order.
    pub customer_id: Option<i32>,
    /// External human-friendly reference for the order.
    pub reference: Option<String>,
    /// Optional notes supplied by the operator.
    pub notes: Option<String>,
    /// Total amount represented in the smallest currency unit (for example cents).
    pub total_cents: i32,
    /// ISO 4217 currency code used for the order total.
    pub currency: String,
    /// Product snapshots captured when the order was created.
    pub products: Vec<OrderProduct>,
    /// Current lifecycle status of the order.
    pub status: OrderStatus,
    /// Timestamp captured when the order payload was created.
    pub updated_at: NaiveDateTime,
}

/// Static snapshot of a product that was added to an order.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderProduct {
    /// Identifier of the original product, if it still exists.
    pub product_id: Option<i32>,
    /// Human-readable name captured at the time of ordering.
    pub name: String,
    /// Stock keeping unit captured at the time of ordering.
    pub sku: Option<String>,
    /// Description captured at the time of ordering.
    pub description: Option<String>,
    /// Price represented in the smallest currency unit for the ordered quantity.
    pub price_cents: i32,
    /// ISO 4217 currency captured at the time of ordering.
    pub currency: String,
    /// Quantity of the product ordered.
    pub quantity: i32,
}

impl OrderProduct {
    /// Create a new ordered product snapshot using the supplied fields.
    pub fn new(
        name: impl Into<String>,
        price_cents: i32,
        currency: impl Into<String>,
        quantity: i32,
    ) -> Self {
        Self {
            product_id: None,
            name: name.into(),
            sku: None,
            description: None,
            price_cents,
            currency: currency.into(),
            quantity,
        }
    }

    /// Associate the snapshot with the current product identifier.
    pub fn with_product_id(mut self, product_id: i32) -> Self {
        self.product_id = Some(product_id);
        self
    }

    /// Capture the SKU value alongside the snapshot.
    pub fn with_sku(mut self, sku: impl Into<String>) -> Self {
        self.sku = Some(sku.into());
        self
    }

    /// Capture the description value alongside the snapshot.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl NewOrder {
    /// Build a new order payload with the supplied details and current timestamp.
    pub fn new(hub_id: i32, total_cents: i32, currency: impl Into<String>) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            customer_id: None,
            reference: None,
            notes: None,
            total_cents,
            currency: currency.into(),
            status: OrderStatus::default(),
            products: Vec::new(),
            updated_at: now,
        }
    }

    /// Attach a customer identifier to the order payload.
    pub fn with_customer_id(mut self, customer_id: i32) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    /// Attach an external reference identifier to the order payload.
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    /// Attach operator notes to the order payload.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
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
    /// Optional status update.
    pub status: OrderStatus,
    /// Optional notes update.
    pub notes: Option<String>,
    /// Optional total amount update.
    pub total_cents: i32,
    /// Optional currency update.
    pub currency: String,
    /// Optional customer reference update.
    pub customer_id: Option<i32>,
    /// Optional external reference update.
    pub reference: Option<String>,
    /// Optional product list update.
    pub products: Option<Vec<OrderProduct>>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

/// Query definition used to list orders for a hub.
#[derive(Debug, Clone)]
pub struct OrderListQuery {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional status filter.
    pub status: Option<OrderStatus>,
    /// Optional customer identifier filter.
    pub customer_id: Option<i32>,
    /// Optional search term that matches the reference or notes.
    pub search: Option<String>,
    /// Optional pagination options applied to the query.
    pub pagination: Option<Pagination>,
}

impl OrderListQuery {
    /// Construct a query that targets all orders belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            status: None,
            customer_id: None,
            search: None,
            pagination: None,
        }
    }

    /// Filter the results by the provided status.
    pub fn status(mut self, status: OrderStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter the results by customer identifier.
    pub fn customer_id(mut self, customer_id: i32) -> Self {
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
