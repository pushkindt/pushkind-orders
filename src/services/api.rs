//! Service helpers serving shell and React-owned orders API data.

use chrono::{NaiveDate, NaiveDateTime};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::check_role;

use crate::domain::order::OrderListQuery;
use crate::domain::tag::TagListQuery;
use crate::domain::types::HubId;
use crate::dto::api::{
    CategoryCollectionDto, CategoryDetailsDto, CategoryTreeNodeDto, CurrentUserDto, IamDto,
    LocalUserCollectionDto, NavigationItemDto, NoAccessPageDto, OrderCollectionDto,
    OrderCollectionFiltersDto, OrderDetailsDto, OrderListItemDto, PriceLevelCollectionDto,
    PriceLevelDetailsDto, PriceLevelEditorOptionsDto, PriceLevelListItemDto, ProductCollectionDto,
    ProductCollectionFiltersDto, ProductDetailsDto, ProductEditorOptionsDto, ProductListItemDto,
    TagCollectionDto, TagDetailsDto, TagListItemDto, VendorCollectionDto, VendorDetailsDto,
    VendorListItemDto, VendorUserListItemDto,
};
use crate::dto::main::IndexQuery;
use crate::dto::price_levels::PriceLevelsQuery;
use crate::dto::products::ProductsQuery;
use crate::dto::tags::TagQuery;
use crate::repository::{
    CategoryReader, CustomerReader, OrderReader, PriceLevelReader, ProductReader, TagReader,
    UserListQuery, UserReader, VendorOrderReader, VendorReader, VendorUserReader,
};
use crate::services::{
    HubAccessScope, ServiceResult, categories as categories_service, ensure_catalog_read_access,
    orders as orders_service, price_levels as price_levels_service, products as products_service,
    resolve_hub_access, tags as tags_service, vendors as vendors_service,
};
use crate::{ADMIN_ACCESS_ROLE, SERVICE_ACCESS_ROLE};

fn build_navigation(user: &AuthenticatedUser) -> Vec<NavigationItemDto> {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Vec::new();
    }

    let mut navigation = vec![
        NavigationItemDto {
            name: "Заказы",
            url: "/",
        },
        NavigationItemDto {
            name: "Товары",
            url: "/products",
        },
        NavigationItemDto {
            name: "Категории",
            url: "/categories",
        },
        NavigationItemDto {
            name: "Цены",
            url: "/price-levels",
        },
        NavigationItemDto {
            name: "Теги",
            url: "/tags",
        },
    ];

    if check_role(ADMIN_ACCESS_ROLE, &user.roles) {
        navigation.push(NavigationItemDto {
            name: "Поставщики",
            url: "/vendors",
        });
    }

    navigation
}

fn build_order_list_query<R>(
    query: &IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<OrderListQuery>
where
    R: UserReader + VendorUserReader + ?Sized,
{
    let access = resolve_hub_access(user, repo)?;
    let hub_id = HubId::new(user.hub_id)?;
    let page = query.page.filter(|page| *page > 0).unwrap_or(1);
    let mut list_query = OrderListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let HubAccessScope::Vendor { vendor_id } = access {
        list_query = list_query.vendor_id(vendor_id);
    }

    if let Some(search) = query.search.as_ref() {
        list_query = list_query.search(search);
    }

    if let Some(status) = query.status.as_deref() {
        let status = crate::domain::order::OrderStatus::try_from(status)?;
        list_query = list_query.status(status);
    }

    if let Some(updated_after) = query.updated_after.as_deref() {
        list_query = list_query.updated_after(parse_filter_date_start(updated_after)?);
    }

    if let Some(updated_before) = query.updated_before.as_deref() {
        list_query = list_query.updated_before(parse_filter_date_end(updated_before)?);
    }

    Ok(list_query)
}

fn parse_filter_date_start(input: &str) -> ServiceResult<NaiveDateTime> {
    let date = NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|_| {
        crate::services::ServiceError::Form("Дата фильтра указана неверно.".to_string())
    })?;
    date.and_hms_opt(0, 0, 0).ok_or_else(|| {
        crate::services::ServiceError::Form("Дата фильтра указана неверно.".to_string())
    })
}

fn parse_filter_date_end(input: &str) -> ServiceResult<NaiveDateTime> {
    let date = NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|_| {
        crate::services::ServiceError::Form("Дата фильтра указана неверно.".to_string())
    })?;
    date.and_hms_opt(23, 59, 59).ok_or_else(|| {
        crate::services::ServiceError::Form("Дата фильтра указана неверно.".to_string())
    })
}

fn build_product_list_query<R>(
    query: &ProductsQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<crate::domain::product::ProductListQuery>
where
    R: UserReader + VendorUserReader + ?Sized,
{
    let access = resolve_hub_access(user, repo)?;
    let hub_id = HubId::new(user.hub_id)?;
    let page = query.page.filter(|page| *page > 0).unwrap_or(1);
    let mut list_query = crate::domain::product::ProductListQuery::new(hub_id)
        .paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let HubAccessScope::Vendor { vendor_id } = access {
        list_query = list_query.with_vendor_id(vendor_id);
    }

    if let Some(search) = query.search.as_ref() {
        list_query = list_query.search(search);
    }

    if query.show_archived {
        list_query = list_query.include_archived();
    }

    Ok(list_query)
}

/// Returns shell data for authenticated users.
///
/// This endpoint intentionally does not require the `orders` role because the
/// React-owned `/na` page also needs shell data.
pub fn get_shell_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> ServiceResult<IamDto> {
    Ok(IamDto {
        current_user: CurrentUserDto::from(user),
        home_url: common_config.auth_service_url.clone(),
        navigation: build_navigation(user),
        local_menu_items: Vec::new(),
    })
}

/// Returns local page data for the orders no-access page.
pub fn get_no_access_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> NoAccessPageDto {
    NoAccessPageDto {
        current_user: CurrentUserDto::from(user),
        home_url: common_config.auth_service_url.clone(),
        required_role: SERVICE_ACCESS_ROLE,
    }
}

/// Returns the canonical order collection data for the React orders dashboard.
pub fn get_order_collection_data<R>(
    query: IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<OrderCollectionDto>
where
    R: OrderReader + UserReader + VendorUserReader + ?Sized,
{
    let list_query = build_order_list_query(&query, user, repo)?;
    let page = list_query
        .pagination
        .as_ref()
        .map(|pagination| pagination.page)
        .unwrap_or(1);
    let per_page = list_query
        .pagination
        .as_ref()
        .map(|pagination| pagination.per_page)
        .unwrap_or(DEFAULT_ITEMS_PER_PAGE);
    let (total_items, orders) = repo.list_orders(list_query)?;

    Ok(OrderCollectionDto::new(
        orders.iter().map(OrderListItemDto::from).collect(),
        page,
        per_page,
        total_items,
        OrderCollectionFiltersDto {
            search: query.search,
            status: query.status,
            updated_after: query.updated_after,
            updated_before: query.updated_before,
        },
    ))
}

/// Returns the canonical order details data for the future React order page.
pub fn get_order_details_data<R>(
    order_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    crm_service_url: &str,
) -> ServiceResult<OrderDetailsDto>
where
    R: OrderReader + CustomerReader + UserReader + VendorUserReader + VendorOrderReader + ?Sized,
{
    let details = orders_service::load_order_details(order_id, user, repo)?;
    Ok(OrderDetailsDto::from_parts(
        &details.order,
        details.customer.as_ref(),
        crm_service_url,
    ))
}

pub fn get_product_collection_data<R>(
    query: ProductsQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ProductCollectionDto>
where
    R: ProductReader
        + PriceLevelReader
        + CategoryReader
        + TagReader
        + UserReader
        + VendorUserReader
        + VendorReader
        + ?Sized,
{
    let list_query = build_product_list_query(&query, user, repo)?;
    let page = list_query
        .pagination
        .as_ref()
        .map(|pagination| pagination.page)
        .unwrap_or(1);
    let per_page = list_query
        .pagination
        .as_ref()
        .map(|pagination| pagination.per_page)
        .unwrap_or(DEFAULT_ITEMS_PER_PAGE);
    let data = products_service::load_products_page(query, user, repo)?;
    Ok(ProductCollectionDto::new(
        data.product_items
            .iter()
            .map(ProductListItemDto::from)
            .collect(),
        page,
        per_page,
        data.total_items,
        ProductCollectionFiltersDto {
            search: data.search,
            show_archived: data.show_archived,
        },
        ProductEditorOptionsDto::from_parts(
            &data.categories,
            &data.tags,
            &data.price_levels,
            &data.vendors,
        ),
    ))
}

pub fn get_product_details_data<R>(
    product_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ProductDetailsDto>
where
    R: ProductReader
        + PriceLevelReader
        + CategoryReader
        + TagReader
        + VendorReader
        + UserReader
        + VendorUserReader
        + ?Sized,
{
    let details = products_service::load_product_details(product_id, user, repo)?;
    Ok(ProductDetailsDto::from_parts(
        &details.product,
        &details.categories,
        &details.tags,
        &details.price_levels,
        &details.vendors,
    ))
}

pub fn get_category_collection_data<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<CategoryCollectionDto>
where
    R: CategoryReader + ?Sized,
{
    let data = categories_service::load_categories(user, repo)?;
    Ok(CategoryCollectionDto {
        items: data
            .tree
            .iter()
            .map(CategoryTreeNodeDto::from_tree_node)
            .collect(),
    })
}

pub fn get_category_details_data<R>(
    category_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<CategoryDetailsDto>
where
    R: CategoryReader + ?Sized,
{
    let category = categories_service::load_category_for_edit(category_id, user, repo)?;
    Ok(CategoryDetailsDto::from_category(&category))
}

pub fn get_tag_collection_data<R>(
    query: TagQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<TagCollectionDto>
where
    R: TagReader + ?Sized,
{
    ensure_catalog_read_access(user)?;
    let page = query.page.filter(|page| *page > 0).unwrap_or(1);
    let mut list_query = TagListQuery::try_new(user.hub_id)?.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(search) = query.search.as_ref() {
        list_query = list_query.search(search);
    }

    let (total_items, tags) = repo.list_tags(list_query)?;

    Ok(TagCollectionDto::new(
        tags.iter().map(TagListItemDto::from_tag).collect(),
        page,
        DEFAULT_ITEMS_PER_PAGE,
        total_items,
        query.search,
    ))
}

pub fn get_tag_details_data<R>(
    tag_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<TagDetailsDto>
where
    R: TagReader + ?Sized,
{
    let tag = tags_service::load_tag_for_edit(tag_id, user, repo)?;
    Ok(TagDetailsDto::from_tag(&tag))
}

pub fn get_price_level_collection_data<R>(
    query: PriceLevelsQuery,
    user: &AuthenticatedUser,
    repo: &R,
    crm_service_url: &str,
) -> ServiceResult<PriceLevelCollectionDto>
where
    R: PriceLevelReader + CategoryReader + ?Sized,
{
    let data = price_levels_service::load_price_levels(query, user, repo)?;

    Ok(PriceLevelCollectionDto {
        items: data
            .price_levels
            .iter()
            .map(PriceLevelListItemDto::from_price_level)
            .collect(),
        active_filters: OrderCollectionFiltersDto {
            search: data.search,
            status: None,
            updated_after: None,
            updated_before: None,
        },
        editor_options: PriceLevelEditorOptionsDto::from_parts(
            &data.price_levels,
            &data.categories,
        ),
        crm_service_url: crm_service_url.to_string(),
    })
}

pub fn get_price_level_details_data<R>(
    price_level_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<PriceLevelDetailsDto>
where
    R: PriceLevelReader + ?Sized,
{
    let price_level = price_levels_service::load_price_level_for_edit(price_level_id, user, repo)?;
    Ok(PriceLevelDetailsDto::from_price_level(&price_level))
}

pub fn get_vendor_collection_data<R>(
    query: crate::dto::vendors::VendorQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<VendorCollectionDto>
where
    R: VendorReader + ?Sized,
{
    vendors_service::ensure_vendors_page_access(user)?;

    let page = query.page.filter(|page| *page > 0).unwrap_or(1);
    let mut list_query = crate::domain::vendor::VendorListQuery::try_new(user.hub_id)?
        .paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(search) = query.search.as_ref() {
        list_query = list_query.search(search);
    }

    let (total_items, vendors) = repo.list_vendors(list_query)?;

    Ok(VendorCollectionDto::new(
        vendors.iter().map(VendorListItemDto::from_vendor).collect(),
        page,
        DEFAULT_ITEMS_PER_PAGE,
        total_items,
        query.search,
    ))
}

pub fn get_vendor_details_data<R>(
    vendor_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<VendorDetailsDto>
where
    R: VendorReader + ?Sized,
{
    let vendor = vendors_service::load_vendor_for_edit(vendor_id, user, repo)?;
    Ok(VendorDetailsDto::from_vendor(&vendor))
}

pub fn get_local_user_collection_data<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<LocalUserCollectionDto>
where
    R: UserReader + VendorReader + VendorUserReader + ?Sized,
{
    vendors_service::ensure_vendors_page_access(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let vendor_lookup = repo
        .list_vendors(crate::domain::vendor::VendorListQuery::new(hub_id))?
        .1
        .into_iter()
        .map(|vendor| (vendor.id, vendor.name.to_string()))
        .collect::<std::collections::HashMap<_, _>>();

    let (_, users) = repo.list_users(UserListQuery::new(hub_id))?;
    let mut items = Vec::new();

    for user_record in users {
        let vendor_id = repo.get_vendor_for_user(user_record.id, hub_id)?;
        let vendor_name = vendor_id.and_then(|id| vendor_lookup.get(&id).cloned());
        items.push(VendorUserListItemDto::from_parts(
            &user_record,
            vendor_id.map(|id| id.get()),
            vendor_name,
        ));
    }

    items.sort_by(|left, right| left.email.cmp(&right.email));

    Ok(LocalUserCollectionDto { items })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 7,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    fn common_config() -> CommonServerConfig {
        CommonServerConfig {
            auth_service_url: "https://auth.example.com".to_string(),
            secret: "supersecret".repeat(8),
        }
    }

    #[test]
    fn shell_data_includes_expected_navigation_for_orders_users() {
        let response = get_shell_data(&sample_user(&["orders"]), &common_config())
            .expect("shell data should succeed");

        let navigation_names = response
            .navigation
            .iter()
            .map(|item| item.name)
            .collect::<Vec<_>>();

        assert_eq!(response.current_user.email, "user@example.com");
        assert_eq!(response.home_url, "https://auth.example.com");
        assert_eq!(
            navigation_names,
            vec!["Заказы", "Товары", "Категории", "Цены", "Теги"]
        );
    }

    #[test]
    fn shell_data_adds_vendors_for_admins() {
        let response = get_shell_data(&sample_user(&["orders", "orders_admin"]), &common_config())
            .expect("shell data should succeed");

        let navigation_names = response
            .navigation
            .iter()
            .map(|item| item.name)
            .collect::<Vec<_>>();

        assert!(navigation_names.contains(&"Поставщики"));
    }

    #[test]
    fn shell_data_keeps_working_without_orders_role() {
        let response = get_shell_data(&sample_user(&["orders_admin"]), &common_config())
            .expect("shell data should still succeed");

        assert_eq!(response.navigation, Vec::<NavigationItemDto>::new());
        assert_eq!(response.local_menu_items, Vec::<NavigationItemDto>::new());
    }

    #[test]
    fn no_access_data_exposes_required_role() {
        let response = get_no_access_data(&sample_user(&[]), &common_config());

        assert_eq!(response.current_user.name, "Tester");
        assert_eq!(response.home_url, "https://auth.example.com");
        assert_eq!(response.required_role, "orders");
    }
}
