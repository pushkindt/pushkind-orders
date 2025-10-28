use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::categories::{AddCategoryForm, EditCategoryForm};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::categories::{
    create_category, load_categories, modify_category, remove_category,
};

#[get("/categories")]
pub async fn show_categories(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_categories(repo.get_ref(), &user) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "categories",
                &server_config.auth_service_url,
            );
            context.insert("category_tree", &data.tree);
            render_template(&tera, "categories/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list categories: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/categories/add")]
pub async fn add_category(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AddCategoryForm>,
) -> impl Responder {
    match create_category(repo.get_ref(), &user, form.into_inner()) {
        Ok(category) => {
            FlashMessage::success(format!("Категория «{}» добавлена.", category.name)).send();
            redirect("/categories")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/categories")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Категория с таким названием уже существует.").send();
            redirect("/categories")
        }
        Err(err) => {
            log::error!("Failed to create category: {err}");
            FlashMessage::error("Не удалось создать категорию.").send();
            redirect("/categories")
        }
    }
}

#[post("/categories/edit")]
pub async fn edit_category(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<EditCategoryForm>,
) -> impl Responder {
    match modify_category(repo.get_ref(), &user, form.into_inner()) {
        Ok(category) => {
            FlashMessage::success(format!("Категория «{}» изменена.", category.name)).send();
            redirect("/categories")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/categories")
        }
        Err(err) => {
            log::error!("Failed to change category: {err}");
            FlashMessage::error("Не удалось изменить категорию.").send();
            redirect("/categories")
        }
    }
}

#[post("/categories/{category_id}/delete")]
pub async fn delete_category(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let category_id = path.into_inner();

    match remove_category(repo.get_ref(), &user, category_id) {
        Ok(()) => {
            FlashMessage::success("Категория удалена.").send();
            redirect("/categories")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Категория не найдена или уже удалена.").send();
            redirect("/categories")
        }
        Err(err) => {
            log::error!("Failed to delete category {category_id}: {err}");
            FlashMessage::error("Не удалось удалить категорию.").send();
            redirect("/categories")
        }
    }
}
