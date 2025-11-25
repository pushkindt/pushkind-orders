use std::sync::Arc;

use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use serde_json::json;

use crate::forms::store::{StoreOrderLinePayload, StoreOtpRequestPayload, StoreOtpVerifyPayload};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::routes::store_session::{get_store_customer_for_hub, set_store_customer};
use crate::services::ServiceError;
use crate::services::store::{
    StoreCategoryFilters, StoreProductFilters, create_store_order, load_store_categories,
    load_store_product, load_store_products, load_store_tags, list_store_orders,
    request_store_otp, verify_store_otp,
};

#[derive(Debug, Deserialize)]
struct HubPath {
    hub_id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreProductsQuery {
    category_id: Option<i32>,
    search: Option<String>,
    page: Option<usize>,
}

impl From<StoreProductsQuery> for StoreProductFilters {
    fn from(value: StoreProductsQuery) -> Self {
        Self {
            category_id: value.category_id,
            search: value.search,
            page: value.page,
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

#[get("/{hub_id}/products")]
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
pub async fn request_store_auth_otp(
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpRequestPayload>,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let zmq_sender = zmq_sender.get_ref().as_ref();

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
pub async fn verify_store_auth_otp(
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpVerifyPayload>,
    repo: web::Data<DieselRepository>,
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

    match verify_store_otp(repo.get_ref(), hub_id, payload.into_inner()) {
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
        Ok(order) => HttpResponse::Created().json(order),
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
