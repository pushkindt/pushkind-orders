//! Data transfer objects for rendering templates and API responses.

pub mod categories;
pub mod main;
pub mod orders;
pub mod price_levels;
pub mod products;
pub mod store;
pub mod tags;

pub use categories::CategoryTreeData;
pub use main::{IndexPageData, IndexQuery};
pub use orders::OrderDetails;
pub use price_levels::{
    ClientPriceLevelAssignment, ClientPriceLevelAssignments, PriceLevelsPageData, PriceLevelsQuery,
};
pub use products::{
    ProductPriceLevelView, ProductTagView, ProductView, ProductsPageData, ProductsQuery,
};
pub use store::{
    StoreCategory, StoreCategoryFilters, StoreOrder, StoreOrderProduct, StoreOtpAcceptResponse,
    StoreOtpVerifyResponse, StoreProduct, StoreProductFilters, StoreTag,
};
pub use tags::{TagQuery, TagsPageData};
