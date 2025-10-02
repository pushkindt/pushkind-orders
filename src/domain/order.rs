use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

/// Possible lifecycle states for an order managed by a hub.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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
    pub total_cents: i64,
    /// ISO 4217 currency code used for the order total.
    pub currency: String,
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
    pub total_cents: i64,
    /// ISO 4217 currency code used for the order total.
    pub currency: String,
    /// Current lifecycle status of the order.
    pub status: OrderStatus,
    /// Timestamp captured when the order payload was created.
    pub updated_at: NaiveDateTime,
}

impl NewOrder {
    /// Build a new order payload with the supplied details and current timestamp.
    pub fn new(hub_id: i32, total_cents: i64, currency: impl Into<String>) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            customer_id: None,
            reference: None,
            notes: None,
            total_cents,
            currency: currency.into(),
            status: OrderStatus::default(),
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
}

/// Patch data applied when updating an existing order.
#[derive(Debug, Clone)]
pub struct UpdateOrder {
    /// Optional status update.
    pub status: Option<OrderStatus>,
    /// Optional notes update.
    pub notes: Option<Option<String>>,
    /// Optional total amount update.
    pub total_cents: Option<i64>,
    /// Optional currency update.
    pub currency: Option<String>,
    /// Optional customer reference update.
    pub customer_id: Option<Option<i32>>,
    /// Optional external reference update.
    pub reference: Option<Option<String>>,
    /// Timestamp captured when the patch was created.
    pub updated_at: NaiveDateTime,
}

impl Default for UpdateOrder {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateOrder {
    /// Create a new patch object with no changes applied yet.
    pub fn new() -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            status: None,
            notes: None,
            total_cents: None,
            currency: None,
            customer_id: None,
            reference: None,
            updated_at: now,
        }
    }

    /// Update the order status.
    pub fn status(mut self, status: OrderStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Update the order notes, using `None` to clear an existing value.
    pub fn notes(mut self, notes: Option<impl Into<String>>) -> Self {
        self.notes = Some(notes.map(|value| value.into()));
        self
    }

    /// Update the total amount of the order.
    pub fn total_cents(mut self, total_cents: i64) -> Self {
        self.total_cents = Some(total_cents);
        self
    }

    /// Update the currency used for the order.
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Update the customer associated with the order, using `None` to clear the value.
    pub fn customer_id(mut self, customer_id: Option<i32>) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    /// Update the external reference associated with the order.
    pub fn reference(mut self, reference: Option<impl Into<String>>) -> Self {
        self.reference = Some(reference.map(|value| value.into()));
        self
    }
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
