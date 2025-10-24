use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::tags::{AddTagForm, EditTagForm};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::tags::{TagQuery, create_tag, load_tags, modify_tag, remove_tag};

#[get("/tags")]
pub async fn show_tags(
    params: web::Query<TagQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_tags(repo.get_ref(), &user, params.0) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "tags",
                &server_config.auth_service_url,
            );
            context.insert("tags", &data.tags);
            context.insert("search", &data.search);
            context.insert("search_action", "/tags");
            render_template(&tera, "tags/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list tags: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/tags/add")]
pub async fn add_tag(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AddTagForm>,
) -> impl Responder {
    match create_tag(repo.get_ref(), &user, form.into_inner()) {
        Ok(tag) => {
            FlashMessage::success(format!("Тег «{}» добавлен.", tag.name)).send();
            redirect("/tags")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/tags")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Тег с таким названием уже существует.").send();
            redirect("/tags")
        }
        Err(err) => {
            log::error!("Failed to create tag: {err}");
            FlashMessage::error("Не удалось создать тег.").send();
            redirect("/tags")
        }
    }
}

#[post("/tags/edit")]
pub async fn edit_tag(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<EditTagForm>,
) -> impl Responder {
    match modify_tag(repo.get_ref(), &user, form.into_inner()) {
        Ok(tag) => {
            FlashMessage::success(format!("Тег «{}» изменен.", tag.name)).send();
            redirect("/tags")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/tags")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Тег с таким названием уже существует.").send();
            redirect("/tags")
        }
        Err(err) => {
            log::error!("Failed to modify tag: {err}");
            FlashMessage::error("Не удалось изменить тег.").send();
            redirect("/tags")
        }
    }
}

#[post("/tags/{tag_id}/delete")]
pub async fn delete_tag(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let tag_id = path.into_inner();

    match remove_tag(repo.get_ref(), &user, tag_id) {
        Ok(()) => {
            FlashMessage::success("Тег удален.").send();
            redirect("/tags")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Тег не найден или уже удален.").send();
            redirect("/tags")
        }
        Err(err) => {
            log::error!("Failed to delete tag {tag_id}: {err}");
            FlashMessage::error("Не удалось удалить тег.").send();
            redirect("/tags")
        }
    }
}
