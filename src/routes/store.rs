use actix_web::{HttpResponse, Responder, get, web};
use log::error;
use serde::Deserialize;

use crate::repository::DieselRepository;
use crate::services::store::{
    StoreCategoryFilters, StoreClientHandle, StoreProductFilters, load_store_categories,
    load_store_products, load_store_tags,
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
