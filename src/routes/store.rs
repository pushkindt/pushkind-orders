use actix_session::Session;
use actix_web::{HttpRequest, HttpResponse, Responder, get, patch, post, web};
use log::{error, info};
use serde::Deserialize;
use serde_json::json;

use crate::dto::store::{StoreCategoryFilters, StoreOrder, StoreProductFilters};
use crate::forms::store::{
    StoreOrderLinePayload, StoreOrderUpdatePayload, StoreOtpRequestPayload, StoreOtpVerifyPayload,
};
use crate::models::config::{ServerConfig, ZmqSenders};
use crate::repository::DieselRepository;
use crate::routes::rate_limit::StoreOtpIpRateLimiter;
use crate::routes::store_session::{get_store_customer_for_hub, set_store_customer};
use crate::services::ServiceError;
use crate::services::store::{
    create_store_order, list_store_orders, load_store_categories, load_store_product,
    load_store_products, load_store_tags, load_store_vendors, request_store_otp,
    update_store_order, verify_store_otp,
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

#[post("/{hub_id}/auth/otp")]
/// Request a one-time password for storefront authentication.
///
/// Sends an SMS with the OTP code to the provided phone number.
pub async fn request_store_auth_otp(
    req: HttpRequest,
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpRequestPayload>,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    server_config: web::Data<ServerConfig>,
    rate_limiter: web::Data<StoreOtpIpRateLimiter>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    if let Some(response) = rate_limit_otp_response(&req, hub_id, rate_limiter.get_ref()) {
        return response;
    }

    let zmq_sender = &zmq_senders.get_ref().sms;

    match request_store_otp(
        hub_id,
        payload.into_inner(),
        repo.get_ref(),
        zmq_sender,
        &server_config.sms_sender,
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

fn rate_limit_otp_response(
    req: &HttpRequest,
    hub_id: i32,
    rate_limiter: &StoreOtpIpRateLimiter,
) -> Option<HttpResponse> {
    let exceeded = match rate_limiter.check(req) {
        Ok(()) => return None,
        Err(err) => err,
    };

    let mut retry_after_seconds = exceeded.retry_after.as_secs();
    if exceeded.retry_after.subsec_nanos() > 0 {
        retry_after_seconds = retry_after_seconds.saturating_add(1);
    }
    retry_after_seconds = retry_after_seconds.max(1);

    info!(
        "Storefront OTP request rate limited for hub {hub_id} from ip {} (retry-after={retry_after_seconds}s)",
        exceeded.ip
    );

    Some(
        HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", retry_after_seconds.to_string()))
            .json(json!({ "error": "rate limit exceeded" })),
    )
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

    match verify_store_otp(hub_id, payload.into_inner(), repo.get_ref(), zmq_sender).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{App, body::to_bytes, http::StatusCode, test};

    use crate::routes::rate_limit::{MAX_REQUESTS, StoreOtpIpRateLimiter};

    #[post("/test/auth/otp")]
    async fn rate_limited_test_endpoint(
        req: HttpRequest,
        rate_limiter: web::Data<StoreOtpIpRateLimiter>,
    ) -> impl Responder {
        if let Some(response) = rate_limit_otp_response(&req, 1, rate_limiter.get_ref()) {
            return response;
        }

        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn rate_limit_otp_response_returns_429_with_retry_after_header() {
        let limiter = StoreOtpIpRateLimiter::new();
        let client_addr = "127.0.0.1:45678".parse().expect("valid socket address");
        let hub_id = 1;
        let req = test::TestRequest::default()
            .peer_addr(client_addr)
            .to_http_request();

        for _ in 0..MAX_REQUESTS {
            assert!(limiter.check(&req).is_ok());
        }

        let response = rate_limit_otp_response(&req, hub_id, &limiter)
            .expect("request should be rate limited");

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = response
            .headers()
            .get("Retry-After")
            .expect("Retry-After header should exist")
            .to_str()
            .expect("Retry-After should be ASCII")
            .parse::<u64>()
            .expect("Retry-After should be numeric");
        assert!(retry_after >= 1);

        let body = to_bytes(response.into_body()).await.expect("response body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["error"], "rate limit exceeded");
    }

    #[actix_web::test]
    async fn rate_limiter_route_returns_429_with_retry_after_header() {
        let limiter = web::Data::new(StoreOtpIpRateLimiter::new());
        let client_addr = "127.0.0.1:45678".parse().expect("valid socket address");
        let seed_req = test::TestRequest::default()
            .peer_addr(client_addr)
            .to_http_request();

        for _ in 0..MAX_REQUESTS {
            assert!(limiter.check(&seed_req).is_ok());
        }

        let app = test::init_service(
            App::new()
                .app_data(limiter.clone())
                .service(rate_limited_test_endpoint),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/test/auth/otp")
            .peer_addr(client_addr)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = resp
            .headers()
            .get("Retry-After")
            .expect("Retry-After header should exist")
            .to_str()
            .expect("Retry-After should be ASCII")
            .parse::<u64>()
            .expect("Retry-After should be numeric");
        assert!(retry_after >= 1);

        let body = to_bytes(resp.into_body()).await.expect("response body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["error"], "rate limit exceeded");
    }
}
