use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::{Context, Tera};

use crate::forms::categories::{AddCategoryForm, EditCategoryForm};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::categories::{
    create_category, load_categories, load_category_for_edit, modify_category, remove_category,
};

#[get("/categories")]
/// Render the categories management page with a hierarchical tree view.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
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
/// Create a new product category.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
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

#[post("/category/{category_id}/edit")]
/// Update an existing product category.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn edit_category(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<EditCategoryForm>,
) -> impl Responder {
    match modify_category(repo.get_ref(), &user, form.into_inner(), path.into_inner()) {
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

#[get("/category/{category_id}/modal")]
/// Render the edit category modal for a specific category.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn show_edit_category_modal(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let category_id = path.into_inner();

    match load_category_for_edit(repo.get_ref(), &user, category_id) {
        Ok(category) => {
            let mut context = Context::new();
            context.insert("category", &category);
            render_template(&tera, "categories/edit_category_modal.html", &context)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load category {category_id} modal: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/category/{category_id}/delete")]
/// Delete a product category by ID.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
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
