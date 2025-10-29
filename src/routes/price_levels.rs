use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::price_levels::{AddPriceLevelForm, EditPriceLevelForm, UploadPriceLevelsForm};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::price_levels::{
    PriceLevelsQuery, create_price_level, import_price_levels, load_price_levels,
    remove_price_level, update_price_level,
};

#[get("/price-levels")]
pub async fn show_price_levels(
    params: web::Query<PriceLevelsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_price_levels(repo.get_ref(), &user, params.0) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "price_levels",
                &common_config.auth_service_url,
            );
            context.insert("price_levels", &data.price_levels);
            context.insert("search", &data.search);
            context.insert("search_action", "/price-levels");
            context.insert("crm_service_url", &server_config.crm_service_url);
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

#[post("/price-levels/add")]
pub async fn add_price_level(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AddPriceLevelForm>,
) -> impl Responder {
    match create_price_level(repo.get_ref(), &user, form.into_inner()) {
        Ok(price_level) => {
            FlashMessage::success(format!("Уровень «{}» добавлен.", price_level.name)).send();
            redirect("/price-levels")
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

#[post("/price-levels/{price_level_id}/edit")]
pub async fn edit_price_level(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<EditPriceLevelForm>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match update_price_level(repo.get_ref(), &user, price_level_id, form.into_inner()) {
        Ok(price_level) => {
            FlashMessage::success(format!("Уровень «{}» обновлен.", price_level.name)).send();
            redirect("/price-levels")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/price-levels")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Уровень не найден или недоступен.").send();
            redirect("/price-levels")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Уровень с таким названием уже существует.").send();
            redirect("/price-levels")
        }
        Err(err) => {
            log::error!("Failed to update price level {price_level_id}: {err}");
            FlashMessage::error("Не удалось обновить уровень цен.").send();
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
        Ok(created) => {
            FlashMessage::success(format!("Загружено уровней цен: {created}.")).send();
            redirect("/price-levels")
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

#[post("/price-levels/{price_level_id}/delete")]
pub async fn delete_price_level(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match remove_price_level(repo.get_ref(), &user, price_level_id) {
        Ok(()) => {
            FlashMessage::success("Уровень удален.").send();
            redirect("/price-levels")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Уровень не найден или уже удален.").send();
            redirect("/price-levels")
        }
        Err(err) => {
            log::error!("Failed to delete price level {price_level_id}: {err}");
            FlashMessage::error("Не удалось удалить уровень цен.").send();
            redirect("/price-levels")
        }
    }
}
