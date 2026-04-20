//! Helpers for serving compiled frontend HTML documents.

use std::path::{Path, PathBuf};

pub use pushkind_common::frontend::{FrontendAssetError, open_frontend_html};

pub const FRONTEND_DIST_DIR: &str = "assets/dist";
pub const FRONTEND_INDEX_DOCUMENT: &str = "app/index.html";
pub const FRONTEND_NO_ACCESS_DOCUMENT: &str = "app/no-access.html";
pub const FRONTEND_ORDER_DOCUMENT: &str = "app/order.html";
pub const FRONTEND_PRODUCTS_DOCUMENT: &str = "app/products.html";
pub const FRONTEND_CATEGORIES_DOCUMENT: &str = "app/categories.html";
pub const FRONTEND_TAGS_DOCUMENT: &str = "app/tags.html";
pub const FRONTEND_PRICE_LEVELS_DOCUMENT: &str = "app/price-levels.html";
pub const FRONTEND_VENDORS_DOCUMENT: &str = "app/vendors.html";

pub fn frontend_document_path(document: &str) -> PathBuf {
    Path::new(FRONTEND_DIST_DIR).join(document)
}
