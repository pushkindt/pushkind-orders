//! JSON API routes used for React-owned shell and orders data.

use std::error::Error;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, delete, get, http::StatusCode, post, put, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;

use crate::dto::api::{
    ApiMutationErrorDto, ApiMutationSuccessDto, CategoryDetailsDto, CategoryMutationSuccessDto,
    OrderDetailsDto, OrderMutationSuccessDto, PriceLevelDetailsDto, PriceLevelMutationSuccessDto,
    ProductDetailsDto, ProductMutationSuccessDto, ProductUploadSuccessDto, TagDetailsDto,
    TagMutationSuccessDto, VendorDetailsDto, VendorMutationSuccessDto,
};
use crate::dto::main::IndexQuery;
use crate::dto::price_levels::PriceLevelsQuery;
use crate::dto::products::ProductsQuery;
use crate::dto::tags::TagQuery;
use crate::dto::vendors::VendorQuery;
use crate::forms::categories::{
    AddCategoryForm, AddCategoryPayload, EditCategoryForm, EditCategoryPayload,
};
use crate::forms::orders::{
    EditOrderForm, EditOrderFormError, EditOrderPayload, UpdateOrderApprovalsForm,
    UpdateOrderApprovalsPayload,
};
use crate::forms::price_levels::{
    AddPriceLevelForm, AddPriceLevelPayload, AssignClientPriceLevelForm,
    AssignClientPriceLevelPayload, EditPriceLevelForm, EditPriceLevelPayload,
};
use crate::forms::products::{
    AddProductForm, AddProductPayload, EditProductForm as EditProductDataForm, EditProductPayload,
    ProductFormError, UploadProductsForm, UploadProductsPayload,
};
use crate::forms::tags::{AddTagForm, AddTagPayload, EditTagForm, EditTagPayload, TagFormError};
use crate::forms::vendors::{
    AddUserForm, AddUserPayload, AddVendorForm, AddVendorPayload, AssignVendorUserForm,
    AssignVendorUserPayload, ClearVendorUserPayload, EditVendorForm, EditVendorPayload,
};
use crate::models::config::AppConfig;
use crate::repository::DieselRepository;
use crate::services::price_levels::{
    assign_price_level_to_client_from_payload, create_price_level_from_payload,
    load_client_price_level_assignments, remove_price_level, update_price_level_from_payload,
};
use crate::services::{
    ServiceError, api as api_service, categories as category_service, orders as order_service,
    products as product_service, tags as tag_service, vendors as vendor_service,
};

fn mutation_error_status(err: &ServiceError) -> StatusCode {
    match err {
        ServiceError::Form(_) | ServiceError::TypeConstraint(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ServiceError::Unauthorized => StatusCode::UNAUTHORIZED,
        ServiceError::NotFound => StatusCode::NOT_FOUND,
        ServiceError::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn mutation_error_dto(err: &ServiceError, not_found_message: &str) -> ApiMutationErrorDto {
    match err {
        ServiceError::Form(message) | ServiceError::TypeConstraint(message) => {
            ApiMutationErrorDto {
                message: message.clone(),
                field_errors: Vec::new(),
            }
        }
        ServiceError::Unauthorized => ApiMutationErrorDto {
            message: "Недостаточно прав.".to_string(),
            field_errors: Vec::new(),
        },
        ServiceError::NotFound => ApiMutationErrorDto {
            message: not_found_message.to_string(),
            field_errors: Vec::new(),
        },
        ServiceError::Conflict => ApiMutationErrorDto {
            message: "Конфликт данных.".to_string(),
            field_errors: Vec::new(),
        },
        _ => ApiMutationErrorDto {
            message: "Внутренняя ошибка сервиса.".to_string(),
            field_errors: Vec::new(),
        },
    }
}

fn mutation_error_response(err: &ServiceError, not_found_message: &str) -> HttpResponse {
    HttpResponse::build(mutation_error_status(err)).json(mutation_error_dto(err, not_found_message))
}

fn order_mutation_success(
    message: impl Into<String>,
    order: crate::dto::orders::OrderDetails,
    app_config: &AppConfig,
) -> OrderMutationSuccessDto {
    OrderMutationSuccessDto {
        message: message.into(),
        order: OrderDetailsDto::from_parts(
            &order.order,
            order.customer.as_ref(),
            &app_config.crm_service_url,
        ),
    }
}

fn product_mutation_success(
    message: impl Into<String>,
    product: &product_service::ProductDetailsPageData,
) -> ProductMutationSuccessDto {
    ProductMutationSuccessDto {
        message: message.into(),
        product: ProductDetailsDto::from_parts(
            &product.product,
            &product.categories,
            &product.tags,
            &product.price_levels,
            &product.vendors,
        ),
    }
}

fn category_mutation_success(
    message: impl Into<String>,
    category: &crate::domain::category::Category,
) -> CategoryMutationSuccessDto {
    CategoryMutationSuccessDto {
        message: message.into(),
        category: CategoryDetailsDto::from_category(category),
    }
}

fn tag_mutation_success(
    message: impl Into<String>,
    tag: &crate::domain::tag::Tag,
) -> TagMutationSuccessDto {
    TagMutationSuccessDto {
        message: message.into(),
        tag: TagDetailsDto::from_tag(tag),
    }
}

fn price_level_mutation_success(
    message: impl Into<String>,
    price_level: &crate::domain::price_level::PriceLevel,
) -> PriceLevelMutationSuccessDto {
    PriceLevelMutationSuccessDto {
        message: message.into(),
        price_level: PriceLevelDetailsDto::from_price_level(price_level),
    }
}

fn vendor_mutation_success(
    message: impl Into<String>,
    vendor: &crate::domain::vendor::Vendor,
) -> VendorMutationSuccessDto {
    VendorMutationSuccessDto {
        message: message.into(),
        vendor: VendorDetailsDto::from_vendor(vendor),
    }
}

#[get("/v1/iam")]
/// Return typed shell data for React-owned orders pages.
pub async fn api_v1_iam(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    match api_service::get_shell_data(&user, common_config.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(err) => {
            log::error!("Failed to load orders shell data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/no-access")]
/// Return typed page data for the React-owned orders no-access page.
pub async fn api_v1_no_access(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    HttpResponse::Ok().json(api_service::get_no_access_data(
        &user,
        common_config.get_ref(),
    ))
}

#[get("/v1/categories")]
pub async fn api_v1_categories(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_category_collection_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list categories: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/categories/{category_id}")]
pub async fn api_v1_category(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let category_id = path.into_inner();

    match api_service::get_category_details_data(category_id, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => {
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            log::error!("Failed to load category {category_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/categories")]
pub async fn api_v1_create_category(
    payload: web::Json<AddCategoryForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddCategoryPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match category_service::create_category_from_payload(payload, &user, repo.get_ref()) {
        Ok(category) => {
            HttpResponse::Ok().json(category_mutation_success("Категория добавлена.", &category))
        }
        Err(err) => {
            log::error!("Failed to create category: {err}");
            mutation_error_response(&err, "Категория не найдена.")
        }
    }
}

#[put("/v1/categories/{category_id}")]
pub async fn api_v1_update_category(
    path: web::Path<i32>,
    payload: web::Json<EditCategoryForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let category_id = path.into_inner();
    let payload: EditCategoryPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match category_service::modify_category_from_payload(
        category_id,
        payload,
        &user,
        repo.get_ref(),
    ) {
        Ok(category) => {
            HttpResponse::Ok().json(category_mutation_success("Категория обновлена.", &category))
        }
        Err(err) => {
            log::error!("Failed to update category {category_id}: {err}");
            mutation_error_response(&err, "Категория не найдена.")
        }
    }
}

#[delete("/v1/categories/{category_id}")]
pub async fn api_v1_delete_category(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let category_id = path.into_inner();

    match category_service::remove_category(category_id, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Категория удалена.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to delete category {category_id}: {err}");
            mutation_error_response(&err, "Категория не найдена.")
        }
    }
}

#[get("/v1/tags")]
pub async fn api_v1_tags(
    params: web::Query<TagQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_tag_collection_data(params.0, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list tags: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/tags/{tag_id}")]
pub async fn api_v1_tag(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let tag_id = path.into_inner();

    match api_service::get_tag_details_data(tag_id, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => {
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            log::error!("Failed to load tag {tag_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/tags")]
pub async fn api_v1_create_tag(
    payload: web::Json<AddTagForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddTagPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match tag_service::create_tag_from_payload(payload, &user, repo.get_ref()) {
        Ok(tag) => HttpResponse::Ok().json(tag_mutation_success("Тег добавлен.", &tag)),
        Err(err) => {
            log::error!("Failed to create tag: {err}");
            mutation_error_response(&err, "Тег не найден.")
        }
    }
}

#[put("/v1/tags/{tag_id}")]
pub async fn api_v1_update_tag(
    path: web::Path<i32>,
    payload: web::Json<EditTagForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let tag_id = path.into_inner();
    let form = payload.into_inner();

    if form.tag_id != tag_id {
        let error = TagFormError::InvalidTagId;
        return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&error));
    }

    let payload: EditTagPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match tag_service::modify_tag_from_payload(payload, &user, repo.get_ref()) {
        Ok(tag) => HttpResponse::Ok().json(tag_mutation_success("Тег обновлён.", &tag)),
        Err(err) => {
            log::error!("Failed to update tag {tag_id}: {err}");
            mutation_error_response(&err, "Тег не найден.")
        }
    }
}

#[delete("/v1/tags/{tag_id}")]
pub async fn api_v1_delete_tag(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let tag_id = path.into_inner();

    match tag_service::remove_tag(tag_id, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Тег удалён.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to delete tag {tag_id}: {err}");
            mutation_error_response(&err, "Тег не найден.")
        }
    }
}

#[get("/v1/price-levels")]
pub async fn api_v1_price_levels(
    params: web::Query<PriceLevelsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    match api_service::get_price_level_collection_data(
        params.0,
        &user,
        repo.get_ref(),
        &app_config.crm_service_url,
    ) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list price levels: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/price-levels/{price_level_id}")]
pub async fn api_v1_price_level(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match api_service::get_price_level_details_data(price_level_id, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => {
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            log::error!("Failed to load price level {price_level_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/price-levels")]
pub async fn api_v1_create_price_level(
    payload: web::Json<AddPriceLevelForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddPriceLevelPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match create_price_level_from_payload(payload, &user, repo.get_ref()) {
        Ok(price_level) => HttpResponse::Ok().json(price_level_mutation_success(
            "Уровень цен добавлен.",
            &price_level,
        )),
        Err(err) => {
            log::error!("Failed to create price level: {err}");
            mutation_error_response(&err, "Уровень цен не найден.")
        }
    }
}

#[put("/v1/price-levels/{price_level_id}")]
pub async fn api_v1_update_price_level(
    path: web::Path<i32>,
    payload: web::Json<EditPriceLevelForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let price_level_id = path.into_inner();
    let payload: EditPriceLevelPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match update_price_level_from_payload(price_level_id, payload, &user, repo.get_ref()) {
        Ok(price_level) => HttpResponse::Ok().json(price_level_mutation_success(
            "Уровень цен обновлён.",
            &price_level,
        )),
        Err(err) => {
            log::error!("Failed to update price level {price_level_id}: {err}");
            mutation_error_response(&err, "Уровень цен не найден.")
        }
    }
}

#[delete("/v1/price-levels/{price_level_id}")]
pub async fn api_v1_delete_price_level(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match remove_price_level(price_level_id, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Уровень цен удалён.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to delete price level {price_level_id}: {err}");
            mutation_error_response(&err, "Уровень цен не найден.")
        }
    }
}

#[get("/v1/vendors")]
pub async fn api_v1_vendors(
    params: web::Query<VendorQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_vendor_collection_data(params.0, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list vendors: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/vendors/{vendor_id}")]
pub async fn api_v1_vendor(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let vendor_id = path.into_inner();

    match api_service::get_vendor_details_data(vendor_id, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => {
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            log::error!("Failed to load vendor {vendor_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/vendors")]
pub async fn api_v1_create_vendor(
    payload: web::Json<AddVendorForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddVendorPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match vendor_service::create_vendor_from_payload(payload, &user, repo.get_ref()) {
        Ok(vendor) => {
            HttpResponse::Ok().json(vendor_mutation_success("Поставщик добавлен.", &vendor))
        }
        Err(err) => {
            log::error!("Failed to create vendor: {err}");
            mutation_error_response(&err, "Поставщик не найден.")
        }
    }
}

#[put("/v1/vendors/{vendor_id}")]
pub async fn api_v1_update_vendor(
    path: web::Path<i32>,
    payload: web::Json<EditVendorForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let vendor_id = path.into_inner();
    let payload: EditVendorPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match vendor_service::modify_vendor_from_payload(vendor_id, payload, &user, repo.get_ref()) {
        Ok(vendor) => {
            HttpResponse::Ok().json(vendor_mutation_success("Поставщик обновлён.", &vendor))
        }
        Err(err) => {
            log::error!("Failed to update vendor {vendor_id}: {err}");
            mutation_error_response(&err, "Поставщик не найден.")
        }
    }
}

#[delete("/v1/vendors/{vendor_id}")]
pub async fn api_v1_delete_vendor(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let vendor_id = path.into_inner();

    match vendor_service::remove_vendor(vendor_id, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Поставщик удалён.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to delete vendor {vendor_id}: {err}");
            mutation_error_response(&err, "Поставщик не найден.")
        }
    }
}

#[get("/v1/users")]
pub async fn api_v1_local_users(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_local_user_collection_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list local users: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/users")]
pub async fn api_v1_create_local_user(
    payload: web::Json<AddUserForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddUserPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match vendor_service::add_user_from_payload(payload, &user, repo.get_ref()) {
        Ok(created_user) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: format!("Пользователь «{}» добавлен.", created_user.email.as_str()),
        }),
        Err(err) => {
            log::error!("Failed to create local user: {err}");
            mutation_error_response(&err, "Пользователь не найден.")
        }
    }
}

#[post("/v1/vendors/assignments")]
pub async fn api_v1_assign_vendor_user(
    payload: web::Json<AssignVendorUserForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AssignVendorUserPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match vendor_service::assign_user_to_vendor_from_payload(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Пользователь привязан к поставщику.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to assign vendor user: {err}");
            mutation_error_response(&err, "Пользователь не найден.")
        }
    }
}

#[delete("/v1/vendors/assignments/{user_id}")]
pub async fn api_v1_clear_vendor_user(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let user_id = path.into_inner();
    let payload =
        match ClearVendorUserPayload::try_from(crate::forms::vendors::ClearVendorUserForm {
            user_id,
        }) {
            Ok(payload) => payload,
            Err(err) => {
                return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
            }
        };

    match vendor_service::clear_vendor_for_user_from_payload(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Привязка пользователя удалена.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to clear vendor user {user_id}: {err}");
            mutation_error_response(&err, "Пользователь не найден.")
        }
    }
}

#[get("/v1/orders")]
/// Return a JSON list of orders with optional search and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_orders(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_order_collection_data(params.0, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list orders: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/orders/{order_id}")]
/// Return typed details for a single order resource.
pub async fn api_v1_order(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    let order_id = path.into_inner();

    match api_service::get_order_details_data(
        order_id,
        &user,
        repo.get_ref(),
        &app_config.crm_service_url,
    ) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => {
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            log::error!("Failed to load order {order_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/v1/orders/{order_id}")]
/// Update editable order metadata and return the refreshed order details resource.
pub async fn api_v1_update_order(
    path: web::Path<i32>,
    payload: web::Json<EditOrderForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    let order_id = path.into_inner();
    let form = payload.into_inner();

    if form.order_id != order_id {
        let error = EditOrderFormError::OrderIdMismatch;
        return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&error));
    }

    let payload: EditOrderPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match order_service::update_order_details(order_id, payload, &user, repo.get_ref()) {
        Ok(order) => HttpResponse::Ok().json(order_mutation_success(
            "Заказ обновлён.",
            order,
            app_config.get_ref(),
        )),
        Err(err) => {
            log::error!("Failed to update order {order_id}: {err}");
            mutation_error_response(&err, "Заказ не найден.")
        }
    }
}

#[put("/v1/orders/{order_id}/products/approvals")]
/// Update approved product quantities and return the refreshed order details resource.
pub async fn api_v1_update_order_product_approvals(
    path: web::Path<i32>,
    payload: web::Json<UpdateOrderApprovalsForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    let order_id = path.into_inner();
    let payload: UpdateOrderApprovalsPayload = match payload.into_inner().try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match order_service::update_order_product_approvals(order_id, payload, &user, repo.get_ref()) {
        Ok(order) => HttpResponse::Ok().json(order_mutation_success(
            "Количество обновлено.",
            order,
            app_config.get_ref(),
        )),
        Err(err) => {
            log::error!("Failed to update order products for {order_id}: {err}");
            mutation_error_response(&err, "Заказ не найден.")
        }
    }
}

#[get("/v1/products")]
/// Return a JSON list of products with optional search, archived filter, and pagination.
pub async fn api_v1_products(
    params: web::Query<ProductsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_product_collection_data(params.0, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list products: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/products/{product_id}")]
/// Return typed details for a single product resource.
pub async fn api_v1_product(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let product_id = path.into_inner();

    match api_service::get_product_details_data(product_id, &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound | ServiceError::TypeConstraint(_)) => {
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            log::error!("Failed to load product {product_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/products")]
/// Create a new product and return the created product resource.
pub async fn api_v1_create_product(
    payload: web::Json<AddProductForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let price_levels =
        match product_service::load_available_price_levels(user.hub_id, repo.get_ref()) {
            Ok(price_levels) => price_levels,
            Err(err) => {
                log::error!("Failed to load price levels before creating product: {err}");
                return mutation_error_response(&err, "Товар не найден.");
            }
        };

    let payload: AddProductPayload =
        match (payload.into_inner(), user.hub_id, &price_levels[..]).try_into() {
            Ok(payload) => payload,
            Err(err) => {
                return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
            }
        };

    match product_service::create_product_from_payload(payload, &user, repo.get_ref()) {
        Ok(product) => {
            match product_service::load_product_details(product.id.get(), &user, repo.get_ref()) {
                Ok(details) => {
                    HttpResponse::Ok().json(product_mutation_success("Товар добавлен.", &details))
                }
                Err(err) => {
                    log::error!(
                        "Failed to reload product {} after creation: {err}",
                        product.id.get()
                    );
                    mutation_error_response(&err, "Товар не найден.")
                }
            }
        }
        Err(err) => {
            log::error!("Failed to create product: {err}");
            mutation_error_response(&err, "Товар не найден.")
        }
    }
}

#[put("/v1/products/{product_id}")]
/// Update an existing product and return the refreshed product resource.
pub async fn api_v1_update_product(
    path: web::Path<i32>,
    payload: web::Json<EditProductDataForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let product_id = path.into_inner();
    let form = payload.into_inner();

    if form.product_id != product_id {
        let error = ProductFormError::ProductIdMismatch;
        return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&error));
    }

    let price_levels =
        match product_service::load_available_price_levels(user.hub_id, repo.get_ref()) {
            Ok(price_levels) => price_levels,
            Err(err) => {
                log::error!(
                    "Failed to load price levels before updating product {product_id}: {err}"
                );
                return mutation_error_response(&err, "Товар не найден.");
            }
        };

    let payload: EditProductPayload = match (form, &price_levels[..]).try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match product_service::update_product_from_payload(product_id, payload, &user, repo.get_ref()) {
        Ok(product) => {
            match product_service::load_product_details(product.id.get(), &user, repo.get_ref()) {
                Ok(details) => {
                    HttpResponse::Ok().json(product_mutation_success("Товар обновлён.", &details))
                }
                Err(err) => {
                    log::error!("Failed to reload product {product_id} after update: {err}");
                    mutation_error_response(&err, "Товар не найден.")
                }
            }
        }
        Err(err) => {
            log::error!("Failed to update product {product_id}: {err}");
            mutation_error_response(&err, "Товар не найден.")
        }
    }
}

#[post("/v1/products/upload")]
/// Upload products from a CSV file and return the number of created records.
pub async fn api_v1_upload_products(
    form: Result<MultipartForm<UploadProductsForm>, Box<dyn Error>>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form = match form {
        Ok(form) => form.0,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto {
                message: format!("Ошибка при обработке формы: {err}"),
                field_errors: vec![crate::dto::api::ApiFieldErrorDto {
                    field: "csv".to_string(),
                    message: format!("Ошибка при обработке формы: {err}"),
                }],
            });
        }
    };

    let price_levels =
        match product_service::load_available_price_levels(user.hub_id, repo.get_ref()) {
            Ok(price_levels) => price_levels,
            Err(err) => {
                log::error!("Failed to load price levels before uploading products: {err}");
                return mutation_error_response(&err, "Товар не найден.");
            }
        };

    let payload: UploadProductsPayload = match (form, user.hub_id, &price_levels[..]).try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match product_service::import_products_from_payload(payload, &user, repo.get_ref()) {
        Ok(created_count) => HttpResponse::Ok().json(ProductUploadSuccessDto {
            message: format!("Загружено товаров: {created_count}."),
            created_count,
        }),
        Err(err) => {
            log::error!("Failed to upload products: {err}");
            mutation_error_response(&err, "Товар не найден.")
        }
    }
}

#[get("/v1/client-price-levels")]
/// Return a JSON list of client price level assignments.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_client_price_levels(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match load_client_price_level_assignments(&user, repo.get_ref()) {
        Ok(assignments) => HttpResponse::Ok().json(assignments),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load client price levels: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/v1/client-price-levels")]
/// Assign a price level to a client.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_update_client_price_level(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    payload: web::Json<AssignClientPriceLevelForm>,
) -> impl Responder {
    let payload = payload.into_inner();
    let log_phone = payload.phone.clone();
    let payload: AssignClientPriceLevelPayload = match payload.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().json(ApiMutationErrorDto::from(&err));
        }
    };

    match assign_price_level_to_client_from_payload(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Уровень цен клиента обновлён.".to_string(),
        }),
        Err(err) => {
            log::error!("Failed to assign price level to client {log_phone}: {err}");
            mutation_error_response(&err, "Клиент не найден.")
        }
    }
}
