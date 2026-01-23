use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::{Context, Tera};

use crate::ADMIN_ACCESS_ROLE;
use crate::dto::price_levels::PriceLevelsQuery;
use crate::forms::price_levels::{AddPriceLevelForm, EditPriceLevelForm};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::price_levels::{
    create_price_level, load_price_level_for_edit, load_price_levels, remove_price_level,
    update_price_level,
};

#[get("/price-levels")]
/// Render the price levels management page with search and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn show_price_levels(
    params: web::Query<PriceLevelsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_price_levels(params.0, &user, repo.get_ref()) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "price_levels",
                &common_config.auth_service_url,
            );
            let is_admin = user.roles.iter().any(|role| role == ADMIN_ACCESS_ROLE);
            context.insert("price_levels", &data.price_levels);
            context.insert("search", &data.search);
            context.insert("categories", &data.categories);
            context.insert("search_action", "/price-levels");
            context.insert("crm_service_url", &server_config.crm_service_url);
            context.insert("is_admin", &is_admin);
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
/// Create a new price level.
///
/// Users without the role stored in `crate::ADMIN_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn add_price_level(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    let form: AddPriceLevelForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/price-levels");
        }
    };

    match create_price_level(form, &user, repo.get_ref()) {
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

#[post("/price-level/{price_level_id}/edit")]
/// Update an existing price level by ID.
///
/// Users without the role stored in `crate::ADMIN_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn edit_price_level(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<EditPriceLevelForm>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match update_price_level(price_level_id, form.into_inner(), &user, repo.get_ref()) {
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

#[get("/price-level/{price_level_id}/modal")]
/// Render the edit price level modal for a specific price level.
///
/// Users without the role stored in `crate::ADMIN_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn show_edit_price_level_modal(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match load_price_level_for_edit(price_level_id, &user, repo.get_ref()) {
        Ok(price_level) => {
            let mut context = Context::new();
            context.insert("price_level", &price_level);
            render_template(&tera, "price_levels/edit_price_modal.html", &context)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load price level {price_level_id} modal: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/price-level/{price_level_id}/delete")]
/// Delete a price level by ID.
///
/// Users without the role stored in `crate::ADMIN_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn delete_price_level(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let price_level_id = path.into_inner();

    match remove_price_level(price_level_id, &user, repo.get_ref()) {
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
