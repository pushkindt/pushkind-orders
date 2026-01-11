use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, patch, post, web};
use log::error;
use serde::Deserialize;
use serde_json::json;

use crate::dto::store::{StoreCategoryFilters, StoreOrder, StoreProductFilters};
use crate::forms::store::{
    StoreOrderLinePayload, StoreOrderUpdatePayload, StoreOtpRequestPayload, StoreOtpVerifyPayload,
};
use crate::models::config::{ServerConfig, ZmqSenders};
use crate::repository::DieselRepository;
use crate::routes::store_session::{get_store_customer_for_hub, set_store_customer};
use crate::services::ServiceError;
use crate::services::store::{
    create_store_order, list_store_orders, load_store_categories, load_store_product,
    load_store_products, load_store_tags, request_store_otp, update_store_order, verify_store_otp,
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
    page: Option<usize>,
}

impl From<StoreProductsQuery> for StoreProductFilters {
    fn from(value: StoreProductsQuery) -> Self {
        Self {
            category_id: value.category_id,
            search: value.search,
            page: value.page,
            tag_id: value.tag_id,
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
    session: Session,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let filters = params
        .map(|query| StoreProductFilters::from(query.into_inner()))
        .unwrap_or_default();
    let store_customer = match get_store_customer_for_hub(&session, hub_id) {
        Ok(customer) => customer,
        Err(err) => {
            error!("Failed to read store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match load_store_products(repo.get_ref(), hub_id, filters, store_customer.as_ref()) {
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
    session: Session,
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

    let store_customer = match get_store_customer_for_hub(&session, hub_id) {
        Ok(customer) => customer,
        Err(err) => {
            error!("Failed to read store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match load_store_product(repo.get_ref(), hub_id, product_id, store_customer.as_ref()) {
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
    match load_store_categories(repo.get_ref(), hub_id, filters) {
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
    match load_store_tags(repo.get_ref(), hub_id) {
        Ok(tags) => HttpResponse::Ok().json(tags),
        Err(err) => {
            error!("Failed to load storefront tags for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/{hub_id}/auth/otp")]
/// Request a one-time password for storefront authentication.
///
/// Sends an SMS with the OTP code to the provided phone number.
pub async fn request_store_auth_otp(
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpRequestPayload>,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let zmq_sender = &zmq_senders.get_ref().sms;

    match request_store_otp(
        repo.get_ref(),
        hub_id,
        zmq_sender,
        &server_config.sms_sender,
        payload.into_inner(),
    )
    .await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(err) => {
            error!("Failed to process OTP request for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/{hub_id}/auth/otp/verify")]
/// Verify a one-time password and establish a store session.
///
/// On success, persists the authenticated customer in the session.
pub async fn verify_store_auth_otp(
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpVerifyPayload>,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    session: Session,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    if let Err(err) = get_store_customer_for_hub(&session, hub_id) {
        error!(
            "Failed to reset mismatched store session before OTP verification for hub {hub_id}: {err}"
        );
        return HttpResponse::InternalServerError().finish();
    }

    let zmq_sender = &zmq_senders.get_ref().clients;

    match verify_store_otp(repo.get_ref(), hub_id, zmq_sender, payload.into_inner()).await {
        Ok(response) => {
            if let Err(err) = set_store_customer(&session, &response.customer) {
                error!("Failed to persist store customer for hub {hub_id}: {err}");
                return HttpResponse::InternalServerError().finish();
            }

            HttpResponse::Ok().json(response)
        }
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(err) => {
            error!("Failed to verify OTP for hub {hub_id}: {err}");
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
    session: Session,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let store_customer = match get_store_customer_for_hub(&session, hub_id) {
        Ok(Some(customer)) => customer,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to read store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match create_store_order(
        repo.get_ref(),
        hub_id,
        &store_customer,
        payload.into_inner(),
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
    session: Session,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let store_customer = match get_store_customer_for_hub(&session, hub_id) {
        Ok(Some(customer)) => customer,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to read store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let page = params.and_then(|query| query.page);

    match list_store_orders(repo.get_ref(), hub_id, &store_customer, page) {
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
    session: Session,
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

    let store_customer = match get_store_customer_for_hub(&session, hub_id) {
        Ok(Some(customer)) => customer,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            error!("Failed to read store session for hub {hub_id}: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let values = match payload.into_inner().into_values() {
        Ok(values) => values,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(json!({ "error": err.to_string() }));
        }
    };

    match update_store_order(repo.get_ref(), hub_id, order_id, &store_customer, values) {
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
