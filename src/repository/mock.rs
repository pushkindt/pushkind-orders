//! Mock repository implementations for testing.

use mockall::mock;

use super::{
    CategoryReader, CategoryWriter, CustomerListQuery, CustomerReader, CustomerWriter, OrderReader,
    OrderWriter, PriceLevelReader, PriceLevelWriter, ProductReader, ProductWriter,
    StoreOtpRepository, TagReader, TagWriter, UserListQuery, UserReader, UserWriter,
};
use crate::domain::{
    category::{Category, CategoryTreeQuery, NewCategory, UpdateCategory},
    customer::{Customer, NewCustomer},
    order::{NewOrder, Order, OrderListQuery, OrderProductApprovalUpdate, UpdateOrder},
    price_level::{NewPriceLevel, PriceLevel, PriceLevelListQuery, UpdatePriceLevel},
    product::{NewProduct, Product, ProductListQuery, UpdateProduct},
    product_price_level::NewProductPriceLevelRate,
    store_otp::{NewStoreOtp, StoreOtp},
    tag::{NewTag, Tag, TagListQuery, UpdateTag},
    types::{
        CategoryId, CategoryName, CustomerId, HubId, ImageUrl, OrderId, PhoneNumber, PriceCents,
        PriceLevelId, ProductId, TagId, UserEmail, UserId,
    },
    user::{NewUser, UpdateUser, User},
};
use pushkind_common::repository::errors::RepositoryResult;

mock! {
    pub ProductReader {}

    impl ProductReader for ProductReader {
        fn get_product_by_id(&self, id: ProductId, hub_id: HubId) -> RepositoryResult<Option<Product>>;
        fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    }
}

mock! {
    pub ProductWriter {}

    impl ProductWriter for ProductWriter {
        fn create_product(&self, new_product: &NewProduct) -> RepositoryResult<Product>;
        fn update_product(&self, product_id: ProductId, hub_id: HubId, updates: &UpdateProduct) -> RepositoryResult<Product>;
        fn delete_product(&self, product_id: ProductId, hub_id: HubId) -> RepositoryResult<()>;
        fn replace_product_price_levels(&self, product_id: ProductId, hub_id: HubId, rates: &[NewProductPriceLevelRate]) -> RepositoryResult<()>;
        fn replace_product_tags(&self, product_id: ProductId, hub_id: HubId, tag_ids: &[TagId]) -> RepositoryResult<()>;
        fn replace_product_images(&self, product_id: ProductId, hub_id: HubId, image_urls: &[ImageUrl]) -> RepositoryResult<()>;
    }
}

mock! {
    pub CustomerReader {}

    impl CustomerReader for CustomerReader {
        fn get_customer_by_id(&self, id: CustomerId, hub_id: HubId) -> RepositoryResult<Option<Customer>>;
        fn get_customer_by_email(&self, email: &UserEmail, hub_id: HubId) -> RepositoryResult<Option<Customer>>;
        fn get_customer_by_phone(&self, phone: &PhoneNumber, hub_id: HubId) -> RepositoryResult<Option<Customer>>;
        fn list_customers(&self, query: CustomerListQuery) -> RepositoryResult<(usize, Vec<Customer>)>;
    }
}

mock! {
    pub CustomerWriter {}

    impl CustomerWriter for CustomerWriter {
        fn create_customer(&self, new_customer: &NewCustomer) -> RepositoryResult<Customer>;
        fn assign_price_level_to_customers(&self, hub_id: HubId, customer_ids: &[CustomerId], price_level_id: Option<PriceLevelId>) -> RepositoryResult<()>;
    }
}

mock! {
    pub PriceLevelReader {}

    impl PriceLevelReader for PriceLevelReader {
        fn get_price_level_by_id(&self, id: PriceLevelId, hub_id: HubId) -> RepositoryResult<Option<PriceLevel>>;
        fn list_price_levels(&self, query: PriceLevelListQuery) -> RepositoryResult<(usize, Vec<PriceLevel>)>;
    }
}

mock! {
    pub PriceLevelWriter {}

    impl PriceLevelWriter for PriceLevelWriter {
        fn create_price_level(&self, new_price_level: &NewPriceLevel) -> RepositoryResult<PriceLevel>;
        fn update_price_level(&self, price_level_id: PriceLevelId, hub_id: HubId, updates: &UpdatePriceLevel) -> RepositoryResult<PriceLevel>;
        fn delete_price_level(&self, price_level_id: PriceLevelId, hub_id: HubId) -> RepositoryResult<()>;
    }
}

mock! {
    pub OrderReader {}

    impl OrderReader for OrderReader {
        fn get_order_by_id(&self, id: OrderId, hub_id: HubId) -> RepositoryResult<Option<Order>>;
        fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<Order>)>;
    }
}

mock! {
    pub OrderWriter {}

    impl OrderWriter for OrderWriter {
        fn create_order(&self, new_order: &NewOrder) -> RepositoryResult<Order>;
        fn update_order(&self, order_id: OrderId, hub_id: HubId, updates: &UpdateOrder) -> RepositoryResult<Order>;
        fn update_order_product_approvals(
            &self,
            order_id: OrderId,
            hub_id: HubId,
            updates: &[OrderProductApprovalUpdate],
            new_total_cents: PriceCents,
            updated_at: chrono::NaiveDateTime,
        ) -> RepositoryResult<Order>;
        fn delete_order(&self, order_id: OrderId, hub_id: HubId) -> RepositoryResult<()>;
    }
}

mock! {
    pub UserReader {}

    impl UserReader for UserReader {
        fn get_user_by_id(&self, id: UserId, hub_id: HubId) -> RepositoryResult<Option<User>>;
        fn get_user_by_email(&self, email: &UserEmail, hub_id: HubId) -> RepositoryResult<Option<User>>;
        fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
    }
}

mock! {
    pub UserWriter {}

    impl UserWriter for UserWriter {
        fn create_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
        fn update_user(&self, user_id: UserId, hub_id: HubId, updates: &UpdateUser) -> RepositoryResult<User>;
        fn delete_user(&self, user_id: UserId, hub_id: HubId) -> RepositoryResult<()>;
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
        fn update_tag(&self, tag_id: TagId, hub_id: HubId, updates: &UpdateTag) -> RepositoryResult<Tag>;
        fn delete_tag(&self, tag_id: TagId, hub_id: HubId) -> RepositoryResult<()>;
    }
}

mock! {
    pub CategoryReader {}

    impl CategoryReader for CategoryReader {
        fn list_categories(&self, query: CategoryTreeQuery) -> RepositoryResult<(usize, Vec<Category>)>;
        fn get_category_by_id(&self, category_id: CategoryId, hub_id: HubId) -> RepositoryResult<Option<Category>>;
        fn get_category_by_name_and_parent(
            &self,
            name: &CategoryName,
            parent_id: Option<CategoryId>,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Category>>;
    }
}

mock! {
    pub CategoryWriter {}

    impl CategoryWriter for CategoryWriter {
        fn create_category(&self, new_category: &NewCategory) -> RepositoryResult<Category>;
        fn update_category(&self, category_id: CategoryId, hub_id: HubId, updates: &UpdateCategory) -> RepositoryResult<Category>;
        fn delete_category(&self, category_id: CategoryId, hub_id: HubId) -> RepositoryResult<()>;
    }
}

mock! {
    pub StoreOtpRepository {}

    impl StoreOtpRepository for StoreOtpRepository {
        fn get_store_otp(&self, hub_id: HubId, phone: &PhoneNumber) -> RepositoryResult<Option<StoreOtp>>;
        fn upsert_store_otp(&self, new_otp: &NewStoreOtp) -> RepositoryResult<StoreOtp>;
        fn delete_store_otp(&self, hub_id: HubId, phone: &PhoneNumber) -> RepositoryResult<()>;
    }
}
