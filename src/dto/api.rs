//! DTOs exposed by React-owned orders API endpoints.

use chrono::NaiveDateTime;
use pushkind_common::domain::auth::AuthenticatedUser;
use serde::Serialize;

use crate::domain::{
    category::Category, customer::Customer, order::Order, order::OrderProduct,
    price_level::PriceLevel, product::Product, tag::Tag, user::User, vendor::Vendor,
};
use crate::dto::products::ProductView;
use crate::forms::FormError;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApiFieldErrorDto {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApiMutationErrorDto {
    pub message: String,
    pub field_errors: Vec<ApiFieldErrorDto>,
}

impl Default for ApiMutationErrorDto {
    fn default() -> Self {
        Self {
            message: "Ошибка валидации формы.".to_string(),
            field_errors: Vec::new(),
        }
    }
}

impl ApiMutationErrorDto {
    fn from_field_errors(
        message: impl Into<String>,
        field_errors: impl IntoIterator<Item = crate::forms::FormFieldError>,
    ) -> Self {
        Self {
            message: message.into(),
            field_errors: field_errors
                .into_iter()
                .map(|error| ApiFieldErrorDto {
                    field: error.field.into_owned(),
                    message: error.message.into_owned(),
                })
                .collect(),
        }
    }
}

impl From<&FormError> for ApiMutationErrorDto {
    fn from(error: &FormError) -> Self {
        Self::from_field_errors(error.to_string(), error.field_errors())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderMutationSuccessDto {
    pub message: String,
    pub order: OrderDetailsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductMutationSuccessDto {
    pub message: String,
    pub product: ProductDetailsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductUploadSuccessDto {
    pub message: String,
    pub created_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApiMutationSuccessDto {
    pub message: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CurrentUserDto {
    pub email: String,
    pub name: String,
    pub hub_id: i32,
    pub roles: Vec<String>,
}

impl From<&AuthenticatedUser> for CurrentUserDto {
    fn from(user: &AuthenticatedUser) -> Self {
        Self {
            email: user.email.clone(),
            name: user.name.clone(),
            hub_id: user.hub_id,
            roles: user.roles.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NavigationItemDto {
    pub name: &'static str,
    pub url: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct IamDto {
    pub current_user: CurrentUserDto,
    pub home_url: String,
    pub navigation: Vec<NavigationItemDto>,
    pub local_menu_items: Vec<NavigationItemDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NoAccessPageDto {
    pub current_user: CurrentUserDto,
    pub home_url: String,
    pub required_role: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderListItemDto {
    pub id: i32,
    pub reference: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub total_cents: i32,
    pub currency: String,
    pub products_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderPaginationDto {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_previous_page: bool,
    pub has_next_page: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderCollectionFiltersDto {
    pub search: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderCollectionDto {
    pub items: Vec<OrderListItemDto>,
    pub pagination: OrderPaginationDto,
    pub active_filters: OrderCollectionFiltersDto,
}

impl OrderCollectionDto {
    pub fn new(
        items: Vec<OrderListItemDto>,
        page: usize,
        per_page: usize,
        total_items: usize,
        active_filters: OrderCollectionFiltersDto,
    ) -> Self {
        let total_pages = total_items.div_ceil(per_page);

        Self {
            items,
            pagination: OrderPaginationDto {
                page,
                per_page,
                total_items,
                total_pages,
                has_previous_page: page > 1,
                has_next_page: page < total_pages,
            },
            active_filters,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderCustomerSummaryDto {
    pub id: i32,
    pub name: String,
    pub phone: String,
    pub public_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderProductItemDto {
    pub product_id: Option<i32>,
    pub name: String,
    pub sku: Option<String>,
    pub quantity: i32,
    pub approved_quantity: i32,
    pub price_cents: i32,
    pub currency: String,
    pub default_price_cents: Option<i32>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OrderDetailsDto {
    pub id: i32,
    pub customer_id: Option<i32>,
    pub reference: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub total_cents: i32,
    pub currency: String,
    pub notes: Option<String>,
    pub shipping_address: Option<String>,
    pub consignee: Option<String>,
    pub delivery_notes: Option<String>,
    pub payer: Option<String>,
    pub customer: Option<OrderCustomerSummaryDto>,
    pub crm_service_url: String,
    pub products: Vec<OrderProductItemDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductNamedOptionDto {
    pub id: i32,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductPriceLevelRateDto {
    pub price_level_id: i32,
    pub price_level_name: String,
    pub price_cents: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductEditorOptionsDto {
    pub categories: Vec<ProductNamedOptionDto>,
    pub tags: Vec<ProductNamedOptionDto>,
    pub price_levels: Vec<ProductNamedOptionDto>,
    pub vendors: Vec<ProductNamedOptionDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductListItemDto {
    pub id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub description_html: Option<String>,
    pub units: Option<String>,
    pub amount: Option<String>,
    pub currency: String,
    pub is_archived: bool,
    pub category: Option<ProductNamedOptionDto>,
    pub vendor: Option<ProductNamedOptionDto>,
    pub updated_at: String,
    pub image_urls: Vec<String>,
    pub tags: Vec<ProductNamedOptionDto>,
    pub price_levels: Vec<ProductPriceLevelRateDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductPaginationDto {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_previous_page: bool,
    pub has_next_page: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductCollectionFiltersDto {
    pub search: Option<String>,
    pub show_archived: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductCollectionDto {
    pub items: Vec<ProductListItemDto>,
    pub pagination: ProductPaginationDto,
    pub active_filters: ProductCollectionFiltersDto,
    pub editor_options: ProductEditorOptionsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProductDetailsDto {
    pub id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub description_html: Option<String>,
    pub units: Option<String>,
    pub amount: Option<String>,
    pub currency: String,
    pub is_archived: bool,
    pub category_id: Option<i32>,
    pub vendor_id: Option<i32>,
    pub tag_ids: Vec<i32>,
    pub image_urls: Vec<String>,
    pub price_levels: Vec<ProductPriceLevelRateDto>,
    pub updated_at: String,
    pub editor_options: ProductEditorOptionsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CategoryTreeNodeDto {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub is_archived: bool,
    pub image_url: Option<String>,
    pub updated_at: String,
    pub children: Vec<CategoryTreeNodeDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CategoryCollectionDto {
    pub items: Vec<CategoryTreeNodeDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CategoryDetailsDto {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub is_archived: bool,
    pub image_url: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CategoryMutationSuccessDto {
    pub message: String,
    pub category: CategoryDetailsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct TagListItemDto {
    pub id: i32,
    pub name: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct TagCollectionDto {
    pub items: Vec<TagListItemDto>,
    pub pagination: ProductPaginationDto,
    pub active_filters: OrderCollectionFiltersDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct TagDetailsDto {
    pub id: i32,
    pub name: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct TagMutationSuccessDto {
    pub message: String,
    pub tag: TagDetailsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PriceLevelListItemDto {
    pub id: i32,
    pub name: String,
    pub is_default: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PriceLevelEditorOptionsDto {
    pub base_price_levels: Vec<ProductNamedOptionDto>,
    pub categories: Vec<ProductNamedOptionDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PriceLevelCollectionDto {
    pub items: Vec<PriceLevelListItemDto>,
    pub active_filters: OrderCollectionFiltersDto,
    pub editor_options: PriceLevelEditorOptionsDto,
    pub crm_service_url: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PriceLevelDetailsDto {
    pub id: i32,
    pub name: String,
    pub is_default: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PriceLevelMutationSuccessDto {
    pub message: String,
    pub price_level: PriceLevelDetailsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VendorListItemDto {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VendorCollectionDto {
    pub items: Vec<VendorListItemDto>,
    pub pagination: ProductPaginationDto,
    pub active_filters: OrderCollectionFiltersDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VendorDetailsDto {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VendorMutationSuccessDto {
    pub message: String,
    pub vendor: VendorDetailsDto,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VendorUserListItemDto {
    pub user_id: i32,
    pub name: String,
    pub email: String,
    pub vendor_id: Option<i32>,
    pub vendor_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LocalUserCollectionDto {
    pub items: Vec<VendorUserListItemDto>,
}

fn format_datetime(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}

impl From<&Order> for OrderListItemDto {
    fn from(order: &Order) -> Self {
        Self {
            id: order.id.get(),
            reference: order.reference.as_ref().map(ToString::to_string),
            status: String::from(order.status),
            created_at: format_datetime(order.created_at),
            updated_at: format_datetime(order.updated_at),
            total_cents: order.total_cents.get(),
            currency: order.currency.to_string(),
            products_count: order.products.len(),
        }
    }
}

impl From<&Customer> for OrderCustomerSummaryDto {
    fn from(customer: &Customer) -> Self {
        Self {
            id: customer.id.get(),
            name: customer.name.to_string(),
            phone: customer.phone.to_string(),
            public_id: customer.public_id.as_ref().map(ToString::to_string),
        }
    }
}

impl From<&OrderProduct> for OrderProductItemDto {
    fn from(product: &OrderProduct) -> Self {
        Self {
            product_id: product.product_id.map(|value| value.get()),
            name: product.name.to_string(),
            sku: product.sku.as_ref().map(ToString::to_string),
            quantity: product.quantity.get(),
            approved_quantity: product.approved_quantity.unwrap_or(product.quantity).get(),
            price_cents: product.price_cents.get(),
            currency: product.currency.to_string(),
            default_price_cents: product.default_price_cents.map(|value| value.get()),
        }
    }
}

impl OrderDetailsDto {
    pub fn from_parts(order: &Order, customer: Option<&Customer>, crm_service_url: &str) -> Self {
        Self {
            id: order.id.get(),
            customer_id: order.customer_id.map(|value| value.get()),
            reference: order.reference.as_ref().map(ToString::to_string),
            status: String::from(order.status),
            created_at: format_datetime(order.created_at),
            updated_at: format_datetime(order.updated_at),
            total_cents: order.total_cents.get(),
            currency: order.currency.to_string(),
            notes: order.notes.as_ref().map(ToString::to_string),
            shipping_address: order.shipping_address.as_ref().map(ToString::to_string),
            consignee: order.consignee.as_ref().map(ToString::to_string),
            delivery_notes: order.delivery_notes.as_ref().map(ToString::to_string),
            payer: order.payer.as_ref().map(ToString::to_string),
            customer: customer.map(OrderCustomerSummaryDto::from),
            crm_service_url: crm_service_url.to_string(),
            products: order
                .products
                .iter()
                .map(OrderProductItemDto::from)
                .collect(),
        }
    }
}

fn format_amount(value: Option<f32>) -> Option<String> {
    value.map(|amount| {
        let mut formatted = format!("{amount:.2}");
        while formatted.contains('.') && formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
        formatted
    })
}

impl From<&Category> for ProductNamedOptionDto {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id.get(),
            name: category.name.to_string(),
        }
    }
}

impl From<&Tag> for ProductNamedOptionDto {
    fn from(tag: &Tag) -> Self {
        Self {
            id: tag.id.get(),
            name: tag.name.to_string(),
        }
    }
}

impl From<&PriceLevel> for ProductNamedOptionDto {
    fn from(price_level: &PriceLevel) -> Self {
        Self {
            id: price_level.id.get(),
            name: price_level.name.to_string(),
        }
    }
}

impl From<&Vendor> for ProductNamedOptionDto {
    fn from(vendor: &Vendor) -> Self {
        Self {
            id: vendor.id.get(),
            name: vendor.name.to_string(),
        }
    }
}

impl ProductEditorOptionsDto {
    pub fn from_parts(
        categories: &[Category],
        tags: &[Tag],
        price_levels: &[PriceLevel],
        vendors: &[Vendor],
    ) -> Self {
        Self {
            categories: categories.iter().map(ProductNamedOptionDto::from).collect(),
            tags: tags.iter().map(ProductNamedOptionDto::from).collect(),
            price_levels: price_levels
                .iter()
                .map(ProductNamedOptionDto::from)
                .collect(),
            vendors: vendors.iter().map(ProductNamedOptionDto::from).collect(),
        }
    }
}

impl From<&ProductView> for ProductListItemDto {
    fn from(product: &ProductView) -> Self {
        Self {
            id: product.id,
            name: product.name.clone(),
            sku: product.sku.clone(),
            description_html: product.description.clone(),
            units: product.units.clone(),
            amount: format_amount(product.amount),
            currency: product.currency.clone(),
            is_archived: product.is_archived,
            category: product
                .category_id
                .zip(product.category_name.clone())
                .map(|(id, name)| ProductNamedOptionDto { id, name }),
            vendor: product
                .vendor_id
                .zip(product.vendor_name.clone())
                .map(|(id, name)| ProductNamedOptionDto { id, name }),
            updated_at: format_datetime(product.updated_at),
            image_urls: product.image_urls.clone(),
            tags: product
                .tags
                .iter()
                .map(|tag| ProductNamedOptionDto {
                    id: tag.id,
                    name: tag.name.clone(),
                })
                .collect(),
            price_levels: product
                .price_levels
                .iter()
                .map(|level| ProductPriceLevelRateDto {
                    price_level_id: level.price_level_id,
                    price_level_name: level.price_level_name.clone(),
                    price_cents: level.price_cents,
                })
                .collect(),
        }
    }
}

impl ProductCollectionDto {
    pub fn new(
        items: Vec<ProductListItemDto>,
        page: usize,
        per_page: usize,
        total_items: usize,
        active_filters: ProductCollectionFiltersDto,
        editor_options: ProductEditorOptionsDto,
    ) -> Self {
        let total_pages = total_items.div_ceil(per_page);

        Self {
            items,
            pagination: ProductPaginationDto {
                page,
                per_page,
                total_items,
                total_pages,
                has_previous_page: page > 1,
                has_next_page: page < total_pages,
            },
            active_filters,
            editor_options,
        }
    }
}

impl ProductDetailsDto {
    pub fn from_parts(
        product: &Product,
        categories: &[Category],
        tags: &[Tag],
        price_levels: &[PriceLevel],
        vendors: &[Vendor],
    ) -> Self {
        let editor_options =
            ProductEditorOptionsDto::from_parts(categories, tags, price_levels, vendors);

        Self {
            id: product.id.get(),
            name: product.name.to_string(),
            sku: product.sku.as_ref().map(ToString::to_string),
            description_html: product.description.as_ref().map(ToString::to_string),
            units: product.units.as_ref().map(ToString::to_string),
            amount: format_amount(product.amount.map(|amount| amount.get())),
            currency: product.currency.to_string(),
            is_archived: product.is_archived,
            category_id: product.category_id.map(|id| id.get()),
            vendor_id: product.vendor_id.map(|id| id.get()),
            tag_ids: product.tags.iter().map(|tag| tag.id.get()).collect(),
            image_urls: product.image_urls.iter().map(ToString::to_string).collect(),
            price_levels: product
                .price_levels
                .iter()
                .filter_map(|rate| {
                    price_levels
                        .iter()
                        .find(|price_level| price_level.id == rate.price_level_id)
                        .map(|price_level| ProductPriceLevelRateDto {
                            price_level_id: price_level.id.get(),
                            price_level_name: price_level.name.to_string(),
                            price_cents: rate.price_cents.get(),
                        })
                })
                .collect(),
            updated_at: format_datetime(product.updated_at),
            editor_options,
        }
    }
}

impl CategoryTreeNodeDto {
    pub fn from_tree_node(node: &crate::domain::category::CategoryTreeNode) -> Self {
        Self {
            id: node.category.id.get(),
            parent_id: node.category.parent_id.map(|value| value.get()),
            name: node.category.name.to_string(),
            description: node.category.description.as_ref().map(ToString::to_string),
            is_archived: node.category.is_archived,
            image_url: node.category.image_url.as_ref().map(ToString::to_string),
            updated_at: format_datetime(node.category.updated_at),
            children: node
                .children
                .iter()
                .map(CategoryTreeNodeDto::from_tree_node)
                .collect(),
        }
    }
}

impl CategoryDetailsDto {
    pub fn from_category(category: &Category) -> Self {
        Self {
            id: category.id.get(),
            parent_id: category.parent_id.map(|value| value.get()),
            name: category.name.to_string(),
            description: category.description.as_ref().map(ToString::to_string),
            is_archived: category.is_archived,
            image_url: category.image_url.as_ref().map(ToString::to_string),
            updated_at: format_datetime(category.updated_at),
        }
    }
}

impl TagListItemDto {
    pub fn from_tag(tag: &Tag) -> Self {
        Self {
            id: tag.id.get(),
            name: tag.name.to_string(),
            updated_at: format_datetime(tag.updated_at),
        }
    }
}

impl TagCollectionDto {
    pub fn new(
        items: Vec<TagListItemDto>,
        page: usize,
        per_page: usize,
        total_items: usize,
        search: Option<String>,
    ) -> Self {
        let total_pages = total_items.div_ceil(per_page);

        Self {
            items,
            pagination: ProductPaginationDto {
                page,
                per_page,
                total_items,
                total_pages,
                has_previous_page: page > 1,
                has_next_page: page < total_pages,
            },
            active_filters: OrderCollectionFiltersDto { search },
        }
    }
}

impl TagDetailsDto {
    pub fn from_tag(tag: &Tag) -> Self {
        Self {
            id: tag.id.get(),
            name: tag.name.to_string(),
            updated_at: format_datetime(tag.updated_at),
        }
    }
}

impl PriceLevelListItemDto {
    pub fn from_price_level(price_level: &PriceLevel) -> Self {
        Self {
            id: price_level.id.get(),
            name: price_level.name.to_string(),
            is_default: price_level.is_default,
            updated_at: format_datetime(price_level.updated_at),
        }
    }
}

impl PriceLevelEditorOptionsDto {
    pub fn from_parts(price_levels: &[PriceLevel], categories: &[Category]) -> Self {
        Self {
            base_price_levels: price_levels
                .iter()
                .map(ProductNamedOptionDto::from)
                .collect(),
            categories: categories.iter().map(ProductNamedOptionDto::from).collect(),
        }
    }
}

impl PriceLevelDetailsDto {
    pub fn from_price_level(price_level: &PriceLevel) -> Self {
        Self {
            id: price_level.id.get(),
            name: price_level.name.to_string(),
            is_default: price_level.is_default,
            updated_at: format_datetime(price_level.updated_at),
        }
    }
}

impl VendorListItemDto {
    pub fn from_vendor(vendor: &Vendor) -> Self {
        Self {
            id: vendor.id.get(),
            name: vendor.name.to_string(),
            created_at: format_datetime(vendor.created_at),
            updated_at: format_datetime(vendor.updated_at),
        }
    }
}

impl VendorCollectionDto {
    pub fn new(
        items: Vec<VendorListItemDto>,
        page: usize,
        per_page: usize,
        total_items: usize,
        search: Option<String>,
    ) -> Self {
        let total_pages = total_items.div_ceil(per_page);

        Self {
            items,
            pagination: ProductPaginationDto {
                page,
                per_page,
                total_items,
                total_pages,
                has_previous_page: page > 1,
                has_next_page: page < total_pages,
            },
            active_filters: OrderCollectionFiltersDto { search },
        }
    }
}

impl VendorDetailsDto {
    pub fn from_vendor(vendor: &Vendor) -> Self {
        Self {
            id: vendor.id.get(),
            name: vendor.name.to_string(),
            created_at: format_datetime(vendor.created_at),
            updated_at: format_datetime(vendor.updated_at),
        }
    }
}

impl VendorUserListItemDto {
    pub fn from_parts(user: &User, vendor_id: Option<i32>, vendor_name: Option<String>) -> Self {
        Self {
            user_id: user.id.get(),
            name: user.name.to_string(),
            email: user.email.to_string(),
            vendor_id,
            vendor_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::domain::customer::Customer;
    use crate::domain::order::{Order, OrderProduct, OrderStatus};
    use crate::domain::types::{
        CurrencyCode, CustomerId, CustomerName, HubId, OrderId, OrderReference, PhoneNumber,
        PriceCents, ProductId, ProductQuantity, ProductSku, PublicId,
    };
    use crate::forms::orders::{
        EditOrderForm, UpdateOrderApprovalItemForm, UpdateOrderApprovalsForm,
        UpdateOrderApprovalsPayload,
    };
    use validator::Validate;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(10, 30, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_order() -> Order {
        Order {
            id: OrderId::new(12).expect("valid order id"),
            hub_id: HubId::new(42).expect("valid hub id"),
            customer_id: Some(CustomerId::new(7).expect("valid customer id")),
            reference: Some(OrderReference::new("ORD-42").expect("valid reference")),
            status: OrderStatus::Pending,
            notes: None,
            total_cents: PriceCents::new(12500).expect("valid price"),
            currency: CurrencyCode::new("RUB").expect("valid currency"),
            products: vec![
                OrderProduct::try_new("Яблоки", 5000, "RUB", 2, Some(6000))
                    .expect("valid product")
                    .with_product_id(ProductId::new(4).expect("valid product id"))
                    .with_sku(ProductSku::new("APL-1").expect("valid sku")),
            ],
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        }
    }

    fn sample_customer() -> Customer {
        Customer {
            id: CustomerId::new(7).expect("valid customer id"),
            hub_id: HubId::new(42).expect("valid hub id"),
            name: CustomerName::new("ООО Ромашка").expect("valid customer name"),
            phone: PhoneNumber::new("+79990000000").expect("valid phone"),
            price_level_id: None,
            public_id: Some(PublicId::new("customer-public-id").expect("valid public id")),
        }
    }

    #[test]
    fn current_user_dto_can_be_built_from_authenticated_user() {
        let user = AuthenticatedUser {
            sub: "user-1".into(),
            email: "user@example.com".into(),
            hub_id: 42,
            name: "User".into(),
            roles: vec!["orders".into()],
            exp: 0,
        };

        let dto = CurrentUserDto::from(&user);

        assert_eq!(dto.email, "user@example.com");
        assert_eq!(dto.name, "User");
        assert_eq!(dto.hub_id, 42);
        assert_eq!(dto.roles, vec!["orders".to_string()]);
    }

    #[test]
    fn order_list_item_dto_is_built_from_order() {
        let dto = OrderListItemDto::from(&sample_order());

        assert_eq!(dto.id, 12);
        assert_eq!(dto.reference.as_deref(), Some("ORD-42"));
        assert_eq!(dto.status, "Pending");
        assert_eq!(dto.created_at, "2024-01-01 10:30");
        assert_eq!(dto.updated_at, "2024-01-01 10:30");
        assert_eq!(dto.total_cents, 12500);
        assert_eq!(dto.currency, "RUB");
        assert_eq!(dto.products_count, 1);
    }

    #[test]
    fn order_collection_dto_construction_tracks_pagination_and_filters() {
        let collection = OrderCollectionDto::new(
            vec![OrderListItemDto::from(&sample_order())],
            2,
            20,
            41,
            OrderCollectionFiltersDto {
                search: Some("ord".to_string()),
            },
        );

        assert_eq!(collection.items.len(), 1);
        assert_eq!(collection.pagination.page, 2);
        assert_eq!(collection.pagination.per_page, 20);
        assert_eq!(collection.pagination.total_items, 41);
        assert_eq!(collection.pagination.total_pages, 3);
        assert!(collection.pagination.has_previous_page);
        assert!(collection.pagination.has_next_page);
        assert_eq!(collection.active_filters.search.as_deref(), Some("ord"));
    }

    #[test]
    fn order_details_dto_construction_flattens_order_and_customer_data() {
        let dto = OrderDetailsDto::from_parts(
            &sample_order(),
            Some(&sample_customer()),
            "https://crm.example.com",
        );

        assert_eq!(dto.id, 12);
        assert_eq!(dto.reference.as_deref(), Some("ORD-42"));
        assert_eq!(dto.status, "Pending");
        assert_eq!(dto.total_cents, 12500);
        assert_eq!(dto.currency, "RUB");
        assert_eq!(
            dto.customer.as_ref().map(|customer| customer.name.as_str()),
            Some("ООО Ромашка")
        );
        assert_eq!(
            dto.customer
                .as_ref()
                .and_then(|customer| customer.public_id.as_deref()),
            Some("customer-public-id")
        );
        assert_eq!(dto.customer_id, Some(7));
        assert_eq!(dto.products.len(), 1);
        assert_eq!(dto.products[0].product_id, Some(4));
        assert_eq!(dto.products[0].name, "Яблоки");
        assert_eq!(dto.products[0].sku.as_deref(), Some("APL-1"));
        assert_eq!(dto.products[0].quantity, 2);
        assert_eq!(
            dto.products[0].approved_quantity,
            ProductQuantity::new(2).expect("valid quantity").get()
        );
        assert_eq!(dto.products[0].price_cents, 5000);
        assert_eq!(dto.products[0].default_price_cents, Some(6000));
        assert_eq!(dto.crm_service_url, "https://crm.example.com");
    }

    #[test]
    fn mutation_error_dto_uses_edit_order_field_errors() {
        let error = EditOrderForm {
            order_id: 0,
            status: String::new(),
            reference: None,
            notes: None,
            shipping_address: None,
            consignee: None,
            delivery_notes: None,
            payer: None,
        }
        .validate()
        .expect_err("form should be invalid");

        let dto = ApiMutationErrorDto::from(&FormError::from(error));

        assert_eq!(
            dto.field_errors,
            vec![
                ApiFieldErrorDto {
                    field: "order_id".to_string(),
                    message: "Идентификатор заказа указан неверно.".to_string(),
                },
                ApiFieldErrorDto {
                    field: "status".to_string(),
                    message: "Выберите статус заказа.".to_string(),
                },
            ]
        );
    }

    #[test]
    fn mutation_error_dto_uses_indexed_approval_field_errors() {
        let error = UpdateOrderApprovalsPayload::try_from(UpdateOrderApprovalsForm {
            approvals: vec![UpdateOrderApprovalItemForm {
                product_id: 1,
                approved_quantity: 0,
            }],
        })
        .expect_err("form should be invalid");

        let dto = ApiMutationErrorDto::from(&error);

        assert_eq!(
            dto.message,
            "Ошибка валидации формы: Количество должно быть положительным целым."
        );
        assert_eq!(
            dto.field_errors,
            vec![ApiFieldErrorDto {
                field: "approvals.0.approved_quantity".to_string(),
                message: "Количество должно быть положительным целым.".to_string(),
            }]
        );
    }
}
