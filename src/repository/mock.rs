use mockall::mock;

use super::{
    OrderReader, OrderWriter, PriceLevelReader, PriceLevelWriter, ProductReader, ProductWriter,
    UserListQuery, UserReader, UserWriter,
};
use crate::domain::{
    order::{NewOrder, Order, OrderListQuery, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, Product, ProductListQuery, UpdateProduct},
    product_price_level::NewProductPriceLevelRate,
    user::{NewUser, UpdateUser, User},
};
use pushkind_common::repository::errors::RepositoryResult;

mock! {
    pub ProductReader {}

    impl ProductReader for ProductReader {
        fn get_product_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Product>>;
        fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    }
}

mock! {
    pub ProductWriter {}

    impl ProductWriter for ProductWriter {
        fn create_product(&self, new_product: &NewProduct) -> RepositoryResult<Product>;
        fn update_product(&self, product_id: i32, hub_id: i32, updates: &UpdateProduct) -> RepositoryResult<Product>;
        fn delete_product(&self, product_id: i32, hub_id: i32) -> RepositoryResult<()>;
        fn replace_product_price_levels(&self, product_id: i32, hub_id: i32, rates: &[NewProductPriceLevelRate]) -> RepositoryResult<()>;
    }
}

mock! {
    pub PriceLevelReader {}

    impl PriceLevelReader for PriceLevelReader {
        fn get_price_level_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<PriceLevel>>;
        fn list_price_levels(&self, query: PriceLevelListQuery) -> RepositoryResult<(usize, Vec<PriceLevel>)>;
    }
}

mock! {
    pub PriceLevelWriter {}

    impl PriceLevelWriter for PriceLevelWriter {
        fn create_price_level(&self, new_price_level: &NewPriceLevel) -> RepositoryResult<PriceLevel>;
        fn update_price_level(&self, price_level_id: i32, hub_id: i32, updates: &UpdatePriceLevel) -> RepositoryResult<PriceLevel>;
        fn delete_price_level(&self, price_level_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub OrderReader {}

    impl OrderReader for OrderReader {
        fn get_order_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Order>>;
        fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<Order>)>;
    }
}

mock! {
    pub OrderWriter {}

    impl OrderWriter for OrderWriter {
        fn create_order(&self, new_order: &NewOrder) -> RepositoryResult<Order>;
        fn update_order(&self, order_id: i32, hub_id: i32, updates: &UpdateOrder) -> RepositoryResult<Order>;
        fn delete_order(&self, order_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub UserReader {}

    impl UserReader for UserReader {
        fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>>;
        fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>>;
        fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
    }
}

mock! {
    pub UserWriter {}

    impl UserWriter for UserWriter {
        fn create_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
        fn update_user(&self, user_id: i32, hub_id: i32, updates: &UpdateUser) -> RepositoryResult<User>;
        fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}
