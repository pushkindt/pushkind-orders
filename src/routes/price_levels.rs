use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::price_levels::{AddPriceLevelForm, UploadPriceLevelsForm};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::price_levels::{
    PriceLevelsQuery, create_price_level, import_price_levels, load_price_levels,
};

#[get("/price-levels")]
pub async fn show_price_levels(
    params: web::Query<PriceLevelsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_price_levels(repo.get_ref(), &user, params.0) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "price_levels",
                &server_config.auth_service_url,
            );
            context.insert("price_levels", &data.price_levels);
            context.insert("search", &data.search);
            context.insert("search_action", &"/price-levels");
            render_template(&tera, "price_levels/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list price levels: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/price-levels")]
pub async fn add_price_level(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AddPriceLevelForm>,
) -> impl Responder {
    match create_price_level(repo.get_ref(), &user, form.into_inner()) {
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
            redirect("/price-levels")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Уровень с таким названием уже существует.").send();
            redirect("/price-levels")
        }
        Err(err) => {
            log::error!("Failed to create price level: {err}");
            FlashMessage::error("Не удалось создать уровень цен.").send();
            redirect("/price-levels")
        }
    }
}

#[post("/price-levels/upload")]
pub async fn upload_price_levels(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(form): MultipartForm<UploadPriceLevelsForm>,
) -> impl Responder {
    match import_price_levels(repo.get_ref(), &user, form) {
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
            redirect("/price-levels")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Некоторые уровни уже существуют.").send();
            redirect("/price-levels")
        }
        Err(err) => {
            log::error!("Failed to import price levels: {err}");
            FlashMessage::error("Не удалось загрузить уровни цен.").send();
            redirect("/price-levels")
        }
    }
}
