use mockall::mock;

use super::{
    OrderReader, OrderWriter, ProductReader, ProductWriter, TemplateListQuery, TemplateReader,
    TemplateWriter, UserListQuery, UserReader, UserWriter,
};
use crate::domain::{
    order::{NewOrder, Order, OrderListQuery, UpdateOrder},
    product::{NewProduct, Product, ProductListQuery, UpdateProduct},
    template::{NewTemplate, Template, UpdateTemplate},
    user::{NewUser, UpdateUser, User},
};
use pushkind_common::repository::errors::RepositoryResult;

mock! {
    pub TemplateReader {}

    impl TemplateReader for TemplateReader {
        fn get_template_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Template>>;
        fn list_templates(&self, query: TemplateListQuery) -> RepositoryResult<(usize, Vec<Template>)>;
    }
}

mock! {
    pub TemplateWriter {}

    impl TemplateWriter for TemplateWriter {
        fn create_templates(&self, new_templates: &[NewTemplate]) -> RepositoryResult<usize>;
        fn update_template(&self, template_id: i32, hub_id: i32, updates: &UpdateTemplate) -> RepositoryResult<Template>;
        fn delete_template(&self, template_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

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
