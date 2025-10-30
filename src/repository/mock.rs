use mockall::mock;

use super::{
    CategoryReader, CategoryWriter, CustomerListQuery, CustomerReader, CustomerWriter, OrderReader,
    OrderWriter, PriceLevelReader, PriceLevelWriter, ProductReader, ProductWriter, TagReader,
    TagWriter, UserListQuery, UserReader, UserWriter,
};
use crate::domain::{
    category::{Category, CategoryTreeQuery, NewCategory, UpdateCategory},
    customer::{Customer, NewCustomer},
    order::{NewOrder, Order, OrderListQuery, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, Product, ProductListQuery, UpdateProduct},
    product_price_level::NewProductPriceLevelRate,
    tag::{NewTag, Tag, TagListQuery, UpdateTag},
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
    pub CustomerReader {}

    impl CustomerReader for CustomerReader {
        fn get_customer_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Customer>>;
        fn get_customer_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<Customer>>;
        fn get_customer_by_email_and_phone<'a>(
            &self,
            email: &'a str,
            phone: Option<&'a str>,
            hub_id: i32,
        ) -> RepositoryResult<Option<Customer>>;
        fn list_customers(&self, query: CustomerListQuery) -> RepositoryResult<(usize, Vec<Customer>)>;
    }
}

mock! {
    pub CustomerWriter {}

    impl CustomerWriter for CustomerWriter {
        fn create_customer(&self, new_customer: &NewCustomer) -> RepositoryResult<Customer>;
        fn assign_price_level_to_customers(&self, hub_id: i32, customer_ids: &[i32], price_level_id: Option<i32>) -> RepositoryResult<()>;
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

mock! {
    pub TagReader {}

    impl TagReader for TagReader {
        fn list_tags(&self, query: TagListQuery) -> RepositoryResult<(usize, Vec<Tag>)>;
    }
}

mock! {
    pub TagWriter {}

    impl TagWriter for TagWriter {
        fn create_tag(&self, new_tag: &NewTag) -> RepositoryResult<Tag>;
        fn update_tag(&self, tag_id: i32, hub_id: i32, updates: &UpdateTag) -> RepositoryResult<Tag>;
        fn delete_tag(&self, tag_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub CategoryReader {}

    impl CategoryReader for CategoryReader {
        fn list_categories(&self, query: CategoryTreeQuery) -> RepositoryResult<(usize, Vec<Category>)>;
        fn get_category_by_id(&self, category_id: i32, hub_id: i32) -> RepositoryResult<Option<Category>>;
    }
}

mock! {
    pub CategoryWriter {}

    impl CategoryWriter for CategoryWriter {
        fn create_category(&self, new_category: &NewCategory) -> RepositoryResult<Category>;
        fn update_category(&self, category_id: i32, hub_id: i32, updates: &UpdateCategory) -> RepositoryResult<Category>;
        fn delete_category(&self, category_id: i32, hub_id: i32) -> RepositoryResult<()>;
        fn assign_child_categories(&self, hub_id: i32, parent_id: i32, child_ids: &[i32]) -> RepositoryResult<Category>;
    }
}
