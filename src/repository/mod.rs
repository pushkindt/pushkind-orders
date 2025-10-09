use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::{
    order::{NewOrder, Order, OrderListQuery, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, Product, ProductListQuery, UpdateProduct},
    user::{NewUser, UpdateUser, User},
};

pub mod order;
pub mod price_level;
pub mod product;
pub mod user;

#[cfg(test)]
pub mod mock;

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

    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

/// Read-only operations over product records.
pub trait ProductReader {
    fn get_product_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Product>>;
    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
}

/// Write operations over product records.
pub trait ProductWriter {
    fn create_product(&self, new_product: &NewProduct) -> RepositoryResult<Product>;
    fn update_product(
        &self,
        product_id: i32,
        hub_id: i32,
        updates: &UpdateProduct,
    ) -> RepositoryResult<Product>;
    fn delete_product(&self, product_id: i32, hub_id: i32) -> RepositoryResult<()>;
}

/// Read-only operations over price level records.
pub trait PriceLevelReader {
    fn get_price_level_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<PriceLevel>>;
    fn list_price_levels(
        &self,
        query: PriceLevelListQuery,
    ) -> RepositoryResult<(usize, Vec<PriceLevel>)>;
}

/// Write operations over price level records.
pub trait PriceLevelWriter {
    fn create_price_level(&self, new_price_level: &NewPriceLevel) -> RepositoryResult<PriceLevel>;
    fn update_price_level(
        &self,
        price_level_id: i32,
        hub_id: i32,
        updates: &UpdatePriceLevel,
    ) -> RepositoryResult<PriceLevel>;
    fn delete_price_level(&self, price_level_id: i32, hub_id: i32) -> RepositoryResult<()>;
}

/// Read-only operations over order records including their products.
pub trait OrderReader {
    fn get_order_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Order>>;
    fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<Order>)>;
}

/// Write operations over order records.
pub trait OrderWriter {
    fn create_order(&self, new_order: &NewOrder) -> RepositoryResult<Order>;
    fn update_order(
        &self,
        order_id: i32,
        hub_id: i32,
        updates: &UpdateOrder,
    ) -> RepositoryResult<Order>;
    fn delete_order(&self, order_id: i32, hub_id: i32) -> RepositoryResult<()>;
}

#[derive(Debug, Clone)]
/// Query definition used to list users for a hub.
pub struct UserListQuery {
    pub hub_id: i32,
    pub search: Option<String>,
    pub pagination: Option<Pagination>,
}

impl UserListQuery {
    /// Construct a query that targets all users belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
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
    fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>>;
    fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>>;
    fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
}

/// Write operations over user records.
pub trait UserWriter {
    fn create_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
    fn update_user(
        &self,
        user_id: i32,
        hub_id: i32,
        updates: &UpdateUser,
    ) -> RepositoryResult<User>;
    fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
}
