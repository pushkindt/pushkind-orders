use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use serde::Deserialize;
use serde_json::json;

use crate::forms::store::{StoreOtpRequestPayload, StoreOtpVerifyPayload};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::store::{
    StoreCategoryFilters, StoreClientHandle, StoreProductFilters, load_store_categories,
    load_store_product, load_store_products, load_store_tags, request_store_otp, verify_store_otp,
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

#[get("/{hub_id}/products")]
pub async fn list_store_products(
    path: web::Path<HubPath>,
    params: Option<web::Query<StoreProductsQuery>>,
    repo: web::Data<DieselRepository>,
    store_client: Option<web::ReqData<StoreClientHandle>>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let filters = params
        .map(|query| StoreProductFilters::from(query.into_inner()))
        .unwrap_or_default();
    match load_store_products(repo.get_ref(), hub_id, filters, store_client.as_deref()) {
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
    store_client: Option<web::ReqData<StoreClientHandle>>,
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

    match load_store_product(repo.get_ref(), hub_id, product_id, store_client.as_deref()) {
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
    store_client: Option<web::ReqData<StoreClientHandle>>,
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
    match load_store_categories(repo.get_ref(), hub_id, filters, store_client.as_deref()) {
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
    store_client: Option<web::ReqData<StoreClientHandle>>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match load_store_tags(repo.get_ref(), hub_id, store_client.as_deref()) {
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
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match request_store_otp(hub_id, payload.into_inner()) {
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
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match verify_store_otp(hub_id, payload.into_inner()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(err) => {
            error!("Failed to verify OTP for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
