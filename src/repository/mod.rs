//! Repository traits and Diesel-backed implementation for data persistence.

use chrono::NaiveDateTime;
use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::customer::{CustomerListQuery, UpdateCustomer};
use crate::domain::types::PriceCents;
use crate::domain::{
    category::{Category, CategoryTreeQuery, NewCategory, UpdateCategory},
    customer::{Customer, NewCustomer},
    order::{NewOrder, Order, OrderListQuery, OrderProductApprovalUpdate, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, Product, ProductListQuery, UpdateProduct},
    product_price_level::NewProductPriceLevelRate,
    store_otp::{NewStoreOtp as DomainNewStoreOtp, StoreOtp as DomainStoreOtp},
    tag::{NewTag, Tag, TagListQuery, UpdateTag},
    types::{
        CategoryId, CategoryName, CustomerId, HubId, ImageUrl, OrderId, PhoneNumber, PriceLevelId,
        ProductId, TagId, UserEmail, UserId,
    },
    user::{NewUser, UpdateUser, User},
};

pub mod category;
pub mod customer;
pub mod order;
pub mod price_level;
pub mod product;
pub mod store_otp;
pub mod tag;
pub mod user;
pub mod vendor;

#[cfg(test)]
pub mod mock;

/// Read-only operations over customer records.
pub trait CustomerReader {
    /// Retrieve a customer by ID within a hub.
    fn get_customer_by_id(
        &self,
        id: CustomerId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Customer>>;
    /// Retrieve a customer by phone number within a hub.
    fn get_customer_by_phone(
        &self,
        phone: &PhoneNumber,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Customer>>;
    /// List customers matching the query with pagination and search.
    fn list_customers(&self, query: CustomerListQuery) -> RepositoryResult<(usize, Vec<Customer>)>;
}

/// Write operations over customer records.
pub trait CustomerWriter {
    /// Create a new customer record.
    fn create_customer(&self, new_customer: &NewCustomer) -> RepositoryResult<Customer>;
    /// Assign a price level to multiple customers.
    fn assign_price_level_to_customers(
        &self,
        hub_id: HubId,
        customer_ids: &[CustomerId],
        price_level_id: Option<PriceLevelId>,
    ) -> RepositoryResult<()>;
    /// Update an existing customer record.
    fn update_customer(
        &self,
        customer_id: CustomerId,
        hub_id: HubId,
        updates: &UpdateCustomer,
    ) -> RepositoryResult<Customer>;
}

/// Persistence operations for storefront OTP records.
pub trait StoreOtpRepository {
    /// Retrieve an OTP record by hub ID and phone number.
    fn get_store_otp(
        &self,
        hub_id: HubId,
        phone: &PhoneNumber,
    ) -> RepositoryResult<Option<DomainStoreOtp>>;
    /// Insert or update an OTP record.
    fn upsert_store_otp(&self, new_otp: &DomainNewStoreOtp) -> RepositoryResult<DomainStoreOtp>;
    /// Delete an OTP record by hub ID and phone number.
    fn delete_store_otp(&self, hub_id: HubId, phone: &PhoneNumber) -> RepositoryResult<()>;
}

#[derive(Clone)]
/// Diesel-backed repository implementation that wraps an r2d2 pool.
pub struct DieselRepository {
    pool: DbPool, // r2d2::Pool is cheap to clone
}

impl DieselRepository {
    /// Create a new repository using the provided connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Acquire a database connection from the pool.
    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

/// Read-only operations over product records.
pub trait ProductReader {
    /// Retrieve a product by ID within a hub.
    fn get_product_by_id(&self, id: ProductId, hub_id: HubId) -> RepositoryResult<Option<Product>>;
    /// List products matching the query with pagination and filters.
    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
}

/// Write operations over product records.
pub trait ProductWriter {
    /// Create a new product record.
    fn create_product(&self, new_product: &NewProduct) -> RepositoryResult<Product>;
    /// Update an existing product record.
    fn update_product(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        updates: &UpdateProduct,
    ) -> RepositoryResult<Product>;
    /// Delete a product record.
    fn delete_product(&self, product_id: ProductId, hub_id: HubId) -> RepositoryResult<()>;
    /// Replace all price level associations for a product.
    fn replace_product_price_levels(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        rates: &[NewProductPriceLevelRate],
    ) -> RepositoryResult<()>;
    /// Batch insert price level associations for multiple products.
    fn create_product_price_levels(
        &self,
        hub_id: HubId,
        rates: &[NewProductPriceLevelRate],
    ) -> RepositoryResult<()>;
    /// Replace all tag associations for a product.
    fn replace_product_tags(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        tag_ids: &[TagId],
    ) -> RepositoryResult<()>;
    /// Replace all image URLs for a product.
    fn replace_product_images(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        image_urls: &[ImageUrl],
    ) -> RepositoryResult<()>;
}

/// Read-only operations over price level records.
pub trait PriceLevelReader {
    /// Retrieve a price level by ID within a hub.
    fn get_price_level_by_id(
        &self,
        id: PriceLevelId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<PriceLevel>>;
    /// List price levels matching the query with pagination and search.
    fn list_price_levels(
        &self,
        query: PriceLevelListQuery,
    ) -> RepositoryResult<(usize, Vec<PriceLevel>)>;
}

/// Write operations over price level records.
pub trait PriceLevelWriter {
    /// Create a new price level record.
    fn create_price_level(&self, new_price_level: &NewPriceLevel) -> RepositoryResult<PriceLevel>;
    /// Update an existing price level record.
    fn update_price_level(
        &self,
        price_level_id: PriceLevelId,
        hub_id: HubId,
        updates: &UpdatePriceLevel,
    ) -> RepositoryResult<PriceLevel>;
    /// Delete a price level record.
    fn delete_price_level(
        &self,
        price_level_id: PriceLevelId,
        hub_id: HubId,
    ) -> RepositoryResult<()>;
}

/// Read-only operations over order records including their products.
pub trait OrderReader {
    /// Retrieve an order by ID within a hub.
    fn get_order_by_id(&self, id: OrderId, hub_id: HubId) -> RepositoryResult<Option<Order>>;
    /// List orders matching the query with pagination and filters.
    fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<Order>)>;
}

/// Write operations over order records.
pub trait OrderWriter {
    /// Create a new order with its products.
    fn create_order(&self, new_order: &NewOrder) -> RepositoryResult<Order>;
    /// Update an existing order and optionally replace its products.
    fn update_order(
        &self,
        order_id: OrderId,
        hub_id: HubId,
        updates: &UpdateOrder,
    ) -> RepositoryResult<Order>;
    /// Update approved quantities for order products and the order total.
    fn update_order_product_approvals(
        &self,
        order_id: OrderId,
        hub_id: HubId,
        updates: &[OrderProductApprovalUpdate],
        new_total_cents: PriceCents,
        updated_at: NaiveDateTime,
    ) -> RepositoryResult<Order>;
    /// Delete an order record.
    fn delete_order(&self, order_id: OrderId, hub_id: HubId) -> RepositoryResult<()>;
}

/// Read-only operations over tag records.
pub trait TagReader {
    /// Retrieve a tag by ID within a hub.
    fn get_tag_by_id(&self, tag_id: TagId, hub_id: HubId) -> RepositoryResult<Option<Tag>>;
    /// List tags matching the query with pagination and search.
    fn list_tags(&self, query: TagListQuery) -> RepositoryResult<(usize, Vec<Tag>)>;
}

/// Write operations over tag records.
pub trait TagWriter {
    /// Create a new tag record.
    fn create_tag(&self, new_tag: &NewTag) -> RepositoryResult<Tag>;
    /// Update an existing tag record.
    fn update_tag(
        &self,
        tag_id: TagId,
        hub_id: HubId,
        updates: &UpdateTag,
    ) -> RepositoryResult<Tag>;
    /// Delete a tag record.
    fn delete_tag(&self, tag_id: TagId, hub_id: HubId) -> RepositoryResult<()>;
}

/// Read operations over category records.
pub trait CategoryReader {
    /// List categories matching the query with pagination and filters.
    fn list_categories(&self, query: CategoryTreeQuery)
    -> RepositoryResult<(usize, Vec<Category>)>;
    /// Retrieve a category by ID within a hub.
    fn get_category_by_id(
        &self,
        category_id: CategoryId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Category>>;
    /// Retrieve a category by name and parent within a hub.
    fn get_category_by_name_and_parent(
        &self,
        name: &CategoryName,
        parent_id: Option<CategoryId>,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Category>>;
}

/// Write operations over category records.
pub trait CategoryWriter {
    /// Create a new category record.
    fn create_category(&self, new_category: &NewCategory) -> RepositoryResult<Category>;
    /// Update an existing category record.
    fn update_category(
        &self,
        category_id: CategoryId,
        hub_id: HubId,
        updates: &UpdateCategory,
    ) -> RepositoryResult<Category>;
    /// Delete a category record.
    fn delete_category(&self, category_id: CategoryId, hub_id: HubId) -> RepositoryResult<()>;
}

#[derive(Debug, Clone)]
/// Query definition used to list users for a hub.
pub struct UserListQuery {
    pub hub_id: HubId,
    pub search: Option<String>,
    pub pagination: Option<Pagination>,
}

impl UserListQuery {
    /// Construct a query that targets all users belonging to `hub_id`.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }

    /// Filter the results by a case-insensitive search on email or name fields.
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

/// Read-only operations over user records.
pub trait UserReader {
    /// Retrieve a user by ID within a hub.
    fn get_user_by_id(&self, id: UserId, hub_id: HubId) -> RepositoryResult<Option<User>>;
    /// Retrieve a user by email within a hub.
    fn get_user_by_email(&self, email: &UserEmail, hub_id: HubId)
    -> RepositoryResult<Option<User>>;
    /// List users matching the query with pagination and search.
    fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
}

/// Write operations over user records.
pub trait UserWriter {
    /// Create a new user record.
    fn create_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
    /// Update an existing user record.
    fn update_user(
        &self,
        user_id: UserId,
        hub_id: HubId,
        updates: &UpdateUser,
    ) -> RepositoryResult<User>;
    /// Delete a user record.
    fn delete_user(&self, user_id: UserId, hub_id: HubId) -> RepositoryResult<()>;
}
