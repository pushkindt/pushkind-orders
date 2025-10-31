use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use serde::Deserialize;
use tera::Tera;

use crate::forms::products::{AddProductForm, EditProductForm, UploadProductsForm};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, products};

#[get("/products")]
pub async fn show_products(
    params: web::Query<products::ProductsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match products::load_products_page(repo.get_ref(), &user, params.0) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "products",
                &server_config.auth_service_url,
            );
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
            context.insert("show_archived", &data.show_archived);
            context.insert("has_active_filters", &has_active_filters);
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
pub async fn add_product(
    req: HttpRequest,
    body: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    // Parse the URL-encoded body using serde_qs so nested arrays deserialize correctly.
    let qs_config = serde_qs::Config::new(5, false);
    let form = match qs_config.deserialize_bytes::<AddProductForm>(body.as_ref()) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!("Failed to parse add product form for {}: {err}", req.path());
            FlashMessage::error("Некорректные данные формы.").send();
            return redirect("/products");
        }
    };

    log::debug!("{form:?}");
    match products::create_product(repo.get_ref(), &user, form) {
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
pub async fn upload_products(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(form): MultipartForm<UploadProductsForm>,
) -> impl Responder {
    match products::import_products(repo.get_ref(), &user, form) {
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

#[derive(Debug, Deserialize)]
struct EditProductPayload {
    product_id: i32,
    #[serde(flatten)]
    form: EditProductForm,
}

#[post("/products/edit")]
pub async fn edit_product(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    payload: web::Form<EditProductPayload>,
) -> impl Responder {
    let payload = payload.into_inner();
    let product_id = payload.product_id;

    match products::update_product(repo.get_ref(), &user, product_id, payload.form) {
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
