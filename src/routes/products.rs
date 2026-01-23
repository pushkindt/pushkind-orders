use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::SERVICE_ACCESS_ROLE;
use crate::dto::products::ProductsQuery;
use crate::forms::products::{AddProductForm, EditProductForm, UploadProductsForm};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, products};

#[get("/products")]
/// Render the products management page with search, filters, and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn show_products(
    params: web::Query<ProductsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match products::load_products_page(params.0, &user, repo.get_ref()) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "products",
                &server_config.auth_service_url,
            );
            let is_admin = user.roles.iter().any(|role| role == SERVICE_ACCESS_ROLE);
            let has_active_filters = data.show_archived
                || data
                    .search
                    .as_ref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);
            context.insert("products", &data.products);
            context.insert("search", &data.search);
            context.insert("search_action", "/products");
            context.insert("price_levels", &data.price_levels);
            context.insert("categories", &data.categories);
            context.insert("tags", &data.tags);
            context.insert("vendors", &data.vendors);
            context.insert("show_archived", &data.show_archived);
            context.insert("has_active_filters", &has_active_filters);
            context.insert("is_admin", &is_admin);
            render_template(&tera, "products/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list products: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/products/add")]
/// Create a new product.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn add_product(
    req: HttpRequest,
    body: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let config = serde_qs::Config::new(5, false);
    let form: AddProductForm = match config.deserialize_bytes(body.as_ref()) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!(
                "Failed to parse edit product form for {}: {err}",
                req.path()
            );
            FlashMessage::error("Некорректные данные формы.").send();
            return redirect("/products");
        }
    };

    match products::create_product(form, &user, repo.get_ref()) {
        Ok(product) => {
            FlashMessage::success(format!("Товар «{}» добавлен.", product.name)).send();
            redirect("/products")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/products")
        }
        Err(err) => {
            log::error!("Failed to create product: {err}");
            FlashMessage::error("Не удалось создать товар.").send();
            redirect("/products")
        }
    }
}

#[post("/products/upload")]
/// Batch upload products from a CSV file.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn upload_products(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(form): MultipartForm<UploadProductsForm>,
) -> impl Responder {
    match products::import_products(form, &user, repo.get_ref()) {
        Ok(created) => {
            FlashMessage::success(format!("Загружено товаров: {created}.")).send();
            redirect("/products")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/products")
        }
        Err(err) => {
            log::error!("Failed to import products: {err}");
            FlashMessage::error("Не удалось загрузить товары.").send();
            redirect("/products")
        }
    }
}

#[post("/products/edit")]
/// Update an existing product.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn edit_product(
    req: HttpRequest,
    body: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let config = serde_qs::Config::new(5, false);
    let form: EditProductForm = match config.deserialize_bytes(body.as_ref()) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!(
                "Failed to parse edit product form for {}: {err}",
                req.path()
            );
            FlashMessage::error("Некорректные данные формы.").send();
            return redirect("/products");
        }
    };

    let product_id = form.product_id;

    match products::update_product(product_id, form, &user, repo.get_ref()) {
        Ok(product) => {
            FlashMessage::success(format!("Товар «{}» обновлён.", product.name)).send();
            redirect("/products")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/products")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Товар не найден или уже удалён.").send();
            redirect("/products")
        }
        Err(err) => {
            log::error!("Failed to update product {product_id}: {err}");
            FlashMessage::error("Не удалось обновить товар.").send();
            redirect("/products")
        }
    }
}
