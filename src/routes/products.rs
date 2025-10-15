use actix_multipart::form::MultipartForm;
use actix_web::post;
use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::products::{AddProductForm, UploadProductsForm};
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

#[post("/products")]
pub async fn add_product(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AddProductForm>,
) -> impl Responder {
    match products::create_product(repo.get_ref(), &user, form.into_inner()) {
        Ok(success) => {
            FlashMessage::success(success.message).send();
            redirect(&success.redirect_to)
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
        Ok(success) => {
            FlashMessage::success(success.message).send();
            redirect(&success.redirect_to)
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
