use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::{Context, Tera};

use crate::dto::vendors::VendorQuery;
use crate::forms::vendors::{
    AddVendorForm, AssignVendorUserForm, ClearVendorUserForm, EditVendorForm,
};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::vendors::{
    assign_user_to_vendor, clear_vendor_for_user, create_vendor, load_vendor_for_edit,
    load_vendors_page, modify_vendor, remove_vendor,
};

#[get("/vendors")]
/// Render the vendors management page with search and pagination.
pub async fn show_vendors(
    params: web::Query<VendorQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match load_vendors_page(params.0, &user, repo.get_ref()) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "vendors",
                &server_config.auth_service_url,
            );
            context.insert("vendors", &data.vendors);
            context.insert("vendor_choices", &data.vendor_choices);
            context.insert("users", &data.users);
            context.insert("search", &data.search);
            context.insert("search_action", "/vendors");
            render_template(&tera, "vendors/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list vendors: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/vendors/add")]
/// Create a new vendor.
pub async fn add_vendor(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AddVendorForm>,
) -> impl Responder {
    match create_vendor(form.into_inner(), &user, repo.get_ref()) {
        Ok(vendor) => {
            FlashMessage::success(format!("Поставщик «{}» добавлен.", vendor.name)).send();
            redirect("/vendors")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/vendors")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Поставщик с таким названием уже существует.").send();
            redirect("/vendors")
        }
        Err(err) => {
            log::error!("Failed to create vendor: {err}");
            FlashMessage::error("Не удалось создать поставщика.").send();
            redirect("/vendors")
        }
    }
}

#[post("/vendors/edit")]
/// Update an existing vendor.
pub async fn edit_vendor(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<EditVendorForm>,
) -> impl Responder {
    match modify_vendor(form.into_inner(), &user, repo.get_ref()) {
        Ok(vendor) => {
            FlashMessage::success(format!("Поставщик «{}» изменен.", vendor.name)).send();
            redirect("/vendors")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/vendors")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error("Поставщик с таким названием уже существует.").send();
            redirect("/vendors")
        }
        Err(err) => {
            log::error!("Failed to modify vendor: {err}");
            FlashMessage::error("Не удалось изменить поставщика.").send();
            redirect("/vendors")
        }
    }
}

#[get("/vendor/{vendor_id}/modal")]
/// Render the edit vendor modal for a specific vendor.
pub async fn show_edit_vendor_modal(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let vendor_id = path.into_inner();

    match load_vendor_for_edit(vendor_id, &user, repo.get_ref()) {
        Ok(vendor) => {
            let mut context = Context::new();
            context.insert("vendor", &vendor);
            render_template(&tera, "vendors/edit_vendor_modal.html", &context)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load vendor {vendor_id} modal: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/vendors/{vendor_id}/delete")]
/// Delete a vendor by ID.
pub async fn delete_vendor(
    path: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let vendor_id = path.into_inner();

    match remove_vendor(vendor_id, &user, repo.get_ref()) {
        Ok(()) => {
            FlashMessage::success("Поставщик удален.").send();
            redirect("/vendors")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Поставщик не найден или уже удален.").send();
            redirect("/vendors")
        }
        Err(err) => {
            log::error!("Failed to delete vendor {vendor_id}: {err}");
            FlashMessage::error("Не удалось удалить поставщика.").send();
            redirect("/vendors")
        }
    }
}

#[post("/vendors/assign")]
/// Assign a user to a vendor.
pub async fn assign_vendor_user(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<AssignVendorUserForm>,
) -> impl Responder {
    match assign_user_to_vendor(form.into_inner(), &user, repo.get_ref()) {
        Ok(()) => {
            FlashMessage::success("Пользователь привязан к поставщику.").send();
            redirect("/vendors")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Conflict) => {
            FlashMessage::error(
                "Пользователь уже связан с другим поставщиком. Сначала снимите привязку.",
            )
            .send();
            redirect("/vendors")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/vendors")
        }
        Err(err) => {
            log::error!("Failed to assign vendor user: {err}");
            FlashMessage::error("Не удалось привязать пользователя.").send();
            redirect("/vendors")
        }
    }
}

#[post("/vendors/clear")]
/// Clear a user vendor assignment.
pub async fn clear_vendor_user(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<ClearVendorUserForm>,
) -> impl Responder {
    match clear_vendor_for_user(form.into_inner(), &user, repo.get_ref()) {
        Ok(()) => {
            FlashMessage::success("Привязка пользователя удалена.").send();
            redirect("/vendors")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/vendors")
        }
        Err(err) => {
            log::error!("Failed to clear vendor user: {err}");
            FlashMessage::error("Не удалось удалить привязку.").send();
            redirect("/vendors")
        }
    }
}
