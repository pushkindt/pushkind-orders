use actix_web::{HttpRequest, HttpResponse, Responder, get, patch, post, web};
use log::error;
use serde::Deserialize;
use serde_json::json;

use crate::domain::store_session::STORE_SESSION_COOKIE_NAME;
use crate::dto::store::{StoreCategoryFilters, StoreOrder, StoreProductFilters};
use crate::forms::store::{StoreOrderLinePayload, StoreOrderUpdatePayload};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::store::{
    create_store_order, decode_store_session_cookie, list_store_orders, load_store_categories,
    load_store_product, load_store_products, load_store_tags, load_store_vendors,
    resolve_store_customer, resolve_store_customer_for_write, update_store_order,
};

#[derive(Debug, Deserialize)]
struct HubPath {
    hub_id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreProductsQuery {
    category_id: Option<i32>,
    tag_id: Option<i32>,
    search: Option<String>,
    min_amount: Option<f32>,
    max_amount: Option<f32>,
    page: Option<usize>,
    vendor_id: Option<i32>,
}

impl From<StoreProductsQuery> for StoreProductFilters {
    fn from(value: StoreProductsQuery) -> Self {
        Self {
            category_id: value.category_id,
            search: value.search,
            page: value.page,
            tag_id: value.tag_id,
            min_amount: value.min_amount,
            max_amount: value.max_amount,
            vendor_id: value.vendor_id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct StoreProductPath {
    hub_id: String,
    product_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreCategoriesQuery {
    parent_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreOrdersQuery {
    page: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct StoreOrderPath {
    hub_id: String,
    order_id: String,
}

#[get("/{hub_id}/products")]
/// Return a JSON list of storefront products with optional filters and pagination.
///
/// Applies customer-specific pricing if a valid store session exists.
pub async fn list_store_products(
    path: web::Path<HubPath>,
    params: Option<web::Query<StoreProductsQuery>>,
    repo: web::Data<DieselRepository>,
    req: HttpRequest,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let filters = params
        .map(|query| StoreProductFilters::from(query.into_inner()))
        .unwrap_or_default();
    let store_customer =
        match read_optional_store_customer(&req, hub_id, repo.get_ref(), &server_config.secret) {
            Ok(customer) => customer,
            Err(err) => {
                error!("Failed to resolve optional store customer for hub {hub_id}: {err}");
                return HttpResponse::InternalServerError().finish();
            }
        };

    match load_store_products(hub_id, filters, store_customer.as_ref(), repo.get_ref()) {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(err) => {
            error!("Failed to load storefront products for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/{hub_id}/products/{product_id}")]
/// Return a single storefront product by ID.
///
/// Applies customer-specific pricing if a valid store session exists.
pub async fn get_store_product(
    path: web::Path<StoreProductPath>,
    repo: web::Data<DieselRepository>,
    req: HttpRequest,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let path = path.into_inner();
    let hub_id = match path.hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let product_id = match path.product_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let store_customer =
        match read_optional_store_customer(&req, hub_id, repo.get_ref(), &server_config.secret) {
            Ok(customer) => customer,
            Err(err) => {
                error!("Failed to resolve optional store customer for hub {hub_id}: {err}");
                return HttpResponse::InternalServerError().finish();
            }
        };

    match load_store_product(hub_id, product_id, store_customer.as_ref(), repo.get_ref()) {
        Ok(Some(product)) => HttpResponse::Ok().json(product),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => {
            error!("Failed to load storefront product {product_id} for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/{hub_id}/categories")]
/// Return a JSON list of storefront categories with optional parent filter.
pub async fn list_store_categories(
    path: web::Path<HubPath>,
    params: Option<web::Query<StoreCategoriesQuery>>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let filters = params
        .map(|query| StoreCategoryFilters {
            parent_id: query.parent_id,
        })
        .unwrap_or_default();
    match load_store_categories(hub_id, filters, repo.get_ref()) {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(err) => {
            error!("Failed to load storefront categories for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/{hub_id}/tags")]
/// Return a JSON list of storefront tags.
pub async fn list_store_tags(
    path: web::Path<HubPath>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match load_store_tags(hub_id, repo.get_ref()) {
        Ok(tags) => HttpResponse::Ok().json(tags),
        Err(err) => {
            error!("Failed to load storefront tags for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/{hub_id}/vendors")]
/// Return a JSON list of storefront vendors.
pub async fn list_store_vendors(
    path: web::Path<HubPath>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match load_store_vendors(hub_id, repo.get_ref()) {
        Ok(vendors) => HttpResponse::Ok().json(vendors),
        Err(err) => {
            error!("Failed to load storefront vendors for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/{hub_id}/orders")]
/// Create a new storefront order.
///
/// Requires a valid store session. Returns `401 Unauthorized` if not authenticated.
pub async fn create_store_order_handler(
    path: web::Path<HubPath>,
    payload: web::Json<Vec<StoreOrderLinePayload>>,
    repo: web::Data<DieselRepository>,
    req: HttpRequest,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let claims = match require_store_session_claims(&req, hub_id, &server_config.secret) {
        Ok(claims) => claims,
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to decode store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let store_customer = match resolve_store_customer_for_write(&claims, repo.get_ref(), true) {
        Ok(Some(customer)) => customer,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to resolve local store customer for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match create_store_order(
        hub_id,
        payload.into_inner(),
        &store_customer,
        repo.get_ref(),
    ) {
        Ok(order) => HttpResponse::Created().json(StoreOrder::from(order)),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to create storefront order for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/{hub_id}/orders")]
/// Return a JSON list of orders for the authenticated storefront customer.
///
/// Requires a valid store session. Returns `401 Unauthorized` if not authenticated.
pub async fn list_store_orders_handler(
    path: web::Path<HubPath>,
    params: Option<web::Query<StoreOrdersQuery>>,
    repo: web::Data<DieselRepository>,
    req: HttpRequest,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let claims = match require_store_session_claims(&req, hub_id, &server_config.secret) {
        Ok(claims) => claims,
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to decode store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let page = params.and_then(|query| query.page);
    let Some(store_customer) = (match resolve_store_customer(&claims, repo.get_ref()) {
        Ok(customer) => customer,
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to resolve local store customer for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }) else {
        return HttpResponse::Ok().json(Vec::<StoreOrder>::new());
    };

    match list_store_orders(hub_id, page, &store_customer, repo.get_ref()) {
        Ok(orders) => HttpResponse::Ok().json(orders),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to list storefront orders for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[patch("/{hub_id}/orders/{order_id}")]
/// Apply editable metadata to an authenticated customer's order.
pub async fn update_store_order_handler(
    path: web::Path<StoreOrderPath>,
    payload: web::Json<StoreOrderUpdatePayload>,
    repo: web::Data<DieselRepository>,
    req: HttpRequest,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let path = path.into_inner();

    let hub_id = match path.hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let order_id = match path.order_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let claims = match require_store_session_claims(&req, hub_id, &server_config.secret) {
        Ok(claims) => claims,
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to decode store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let store_customer = match resolve_store_customer_for_write(&claims, repo.get_ref(), false) {
        Ok(Some(customer)) => customer,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to resolve local store customer for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let values = match payload.into_inner().into_values() {
        Ok(values) => values,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(json!({ "error": err.to_string() }));
        }
    };

    match update_store_order(hub_id, order_id, values, &store_customer, repo.get_ref()) {
        Ok(order) => HttpResponse::Ok().json(order),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(err) => {
            error!("Failed to update storefront order {order_id} for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn require_store_session_claims(
    req: &HttpRequest,
    hub_id: i32,
    secret: &str,
) -> Result<crate::domain::store_session::StoreSessionClaims, ServiceError> {
    let cookie = req
        .cookie(STORE_SESSION_COOKIE_NAME)
        .ok_or(ServiceError::Unauthorized)?;
    decode_store_session_cookie(cookie.value(), hub_id, secret)
}

fn read_optional_store_customer(
    req: &HttpRequest,
    hub_id: i32,
    repo: &DieselRepository,
    secret: &str,
) -> Result<Option<crate::domain::customer::Customer>, ServiceError> {
    let claims = req
        .cookie(STORE_SESSION_COOKIE_NAME)
        .and_then(|cookie| decode_store_session_cookie(cookie.value(), hub_id, secret).ok());

    match claims {
        Some(claims) => resolve_store_customer(&claims, repo),
        None => Ok(None),
    }
}
