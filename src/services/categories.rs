use std::collections::{HashMap, HashSet};

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::Deserialize;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::category::{Category, CategoryTreeQuery};
use crate::forms::categories::{AddCategoryForm, AssignChildCategoriesForm, EditCategoryForm};
use crate::repository::{CategoryReader, CategoryWriter};
use crate::services::{ServiceError, ServiceResult};

/// Query parameters accepted by the categories index page.
#[derive(Debug, Default, Deserialize)]
pub struct CategoryQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
    /// Whether archived entries should be included in the response.
    #[serde(default)]
    pub show_archived: bool,
}

/// Data required to render the categories index template.
pub struct CategoriesPageData {
    /// Paginated list of categories displayed in the table.
    pub categories: Paginated<Category>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
    /// Whether archived items were requested.
    pub show_archived: bool,
}

/// Loads the categories overview page.
pub fn load_categories<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: CategoryQuery,
) -> ServiceResult<CategoriesPageData>
where
    R: CategoryReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let CategoryQuery {
        search,
        page,
        show_archived,
    } = query;

    let page = page.unwrap_or(1);
    let mut list_query = CategoryTreeQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(term) = search.as_ref() {
        list_query = list_query.search(term);
    }

    if show_archived {
        list_query = list_query.include_archived();
    }

    let (total, categories) = repo
        .list_categories(list_query)
        .map_err(ServiceError::from)?;

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let categories = Paginated::new(categories, page, total_pages);

    Ok(CategoriesPageData {
        categories,
        search,
        show_archived,
    })
}

/// Creates a new category for the authenticated user's hub.
pub fn create_category<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddCategoryForm,
) -> ServiceResult<Category>
where
    R: CategoryWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let new_category = form
        .into_new_category(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_category(&new_category)
        .map_err(ServiceError::from)
}

/// Assigns a set of child categories to a parent category.
pub fn assign_child_categories<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AssignChildCategoriesForm,
) -> ServiceResult<Category>
where
    R: CategoryWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let payload = form.into_payload();
    if payload.parent_id <= 0 {
        return Err(ServiceError::Form("Некорректная категория.".to_string()));
    }

    let mut unique_children = HashSet::new();
    let mut child_ids = Vec::new();
    for child in payload.child_ids {
        if child <= 0 || child == payload.parent_id {
            continue;
        }
        if unique_children.insert(child) {
            child_ids.push(child);
        }
    }

    repo.assign_child_categories(user.hub_id, payload.parent_id, &child_ids)
        .map_err(ServiceError::from)
}

/// Updates an existing category for the authenticated user's hub.
pub fn modify_category<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: EditCategoryForm,
) -> ServiceResult<Category>
where
    R: CategoryReader + CategoryWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let payload = form
        .into_update_category()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    if matches!(payload.update.parent_id, Some(value) if value == payload.category_id) {
        return Err(ServiceError::Form(
            "Категория не может быть родителем самой себя.".to_string(),
        ));
    }

    if let Some(new_parent_id) = payload.update.parent_id {
        let (_, categories) = repo
            .list_categories(CategoryTreeQuery::new(user.hub_id).include_archived())
            .map_err(ServiceError::from)?;

        let mut children_by_parent: HashMap<i32, Vec<i32>> = HashMap::new();
        for category in &categories {
            if let Some(parent_id) = category.parent_id {
                children_by_parent
                    .entry(parent_id)
                    .or_default()
                    .push(category.id);
            }
        }

        let mut stack = vec![payload.category_id];
        let mut visited = HashSet::new();

        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                if let Some(children) = children_by_parent.get(&current) {
                    stack.extend(children.iter().copied());
                }
            }
        }

        if visited.contains(&new_parent_id) {
            return Err(ServiceError::Form(
                "Категория не может быть родителем своей дочерней категории.".to_string(),
            ));
        }
    }

    repo.update_category(payload.category_id, user.hub_id, &payload.update)
        .map_err(ServiceError::from)
}

/// Deletes a category for the authenticated user's hub.
pub fn remove_category<R>(repo: &R, user: &AuthenticatedUser, category_id: i32) -> ServiceResult<()>
where
    R: CategoryWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    repo.delete_category(category_id, user.hub_id)
        .map_err(ServiceError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::Value;

    use crate::domain::category::{NewCategory as DomainNewCategory, UpdateCategory as DomainUpdateCategory};
    use crate::forms::categories::AssignChildCategoriesForm;
    use crate::repository::mock::{MockCategoryReader, MockCategoryWriter};
    use pushkind_common::repository::errors::RepositoryResult;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    struct MockCategoryRepo {
        pub reader: MockCategoryReader,
        pub writer: MockCategoryWriter,
    }

    impl MockCategoryRepo {
        fn new() -> Self {
            Self {
                reader: MockCategoryReader::new(),
                writer: MockCategoryWriter::new(),
            }
        }
    }

    impl CategoryReader for MockCategoryRepo {
        fn list_categories(&self, query: CategoryTreeQuery) -> RepositoryResult<(usize, Vec<Category>)> {
            self.reader.list_categories(query)
        }

        fn get_category_by_id(
            &self,
            category_id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Option<Category>> {
            self.reader.get_category_by_id(category_id, hub_id)
        }
    }

    impl CategoryWriter for MockCategoryRepo {
        fn create_category(&self, new_category: &DomainNewCategory) -> RepositoryResult<Category> {
            self.writer.create_category(new_category)
        }

        fn update_category(
            &self,
            category_id: i32,
            hub_id: i32,
            updates: &DomainUpdateCategory,
        ) -> RepositoryResult<Category> {
            self.writer.update_category(category_id, hub_id, updates)
        }

        fn delete_category(&self, category_id: i32, hub_id: i32) -> RepositoryResult<()> {
            self.writer.delete_category(category_id, hub_id)
        }

        fn assign_child_categories(
            &self,
            hub_id: i32,
            parent_id: i32,
            child_ids: &[i32],
        ) -> RepositoryResult<Category> {
            self.writer.assign_child_categories(hub_id, parent_id, child_ids)
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 9,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    fn sample_category(id: i32, hub_id: i32, name: &str) -> Category {
        Category {
            id,
            hub_id,
            parent_id: None,
            name: name.to_string(),
            description: None,
            is_archived: false,
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

    #[test]
    fn load_categories_requires_role() {
        let repo = MockCategoryReader::new();
        let user = user_with_roles(&[]);

        let result = load_categories(&repo, &user, CategoryQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_categories_returns_paginated_data() {
        let mut repo = MockCategoryReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = CategoryQuery {
            search: Some("veg".to_string()),
            page: Some(2),
            show_archived: true,
        };
        let expected_hub = user.hub_id;

        repo.expect_list_categories()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id, expected_hub);
                assert!(query.include_archived);
                assert_eq!(query.search.as_deref(), Some("veg"));
                match &query.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 2);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be set"),
                }
                true
            })
            .returning(move |_| {
                Ok((
                    30,
                    vec![
                        sample_category(1, expected_hub, "Vegetables"),
                        sample_category(2, expected_hub, "Vegan"),
                    ],
                ))
            });

        let result = load_categories(&repo, &user, query);
        let data = result.expect("expected success");

        assert!(data.show_archived);
        assert_eq!(data.search.as_deref(), Some("veg"));

        let serialized =
            serde_json::to_value(&data.categories).expect("serialization should succeed");

        let page_value = serialized
            .get("page")
            .and_then(Value::as_u64)
            .expect("expected page");
        assert_eq!(page_value, 2);

        let items = serialized
            .get("items")
            .and_then(Value::as_array)
            .expect("expected items array");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn create_category_requires_role() {
        let repo = MockCategoryWriter::new();
        let user = user_with_roles(&[]);
        let form = AddCategoryForm {
            name: "Retail".to_string(),
            description: None,
            parent_id: None,
        };

        let result = create_category(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn create_category_validates_form() {
        let repo = MockCategoryWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddCategoryForm {
            name: "   ".to_string(),
            description: None,
            parent_id: None,
        };

        let result = create_category(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn create_category_persists_new_entry() {
        let mut repo = MockCategoryWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_create_category()
            .times(1)
            .withf(|new_category| {
                assert_eq!(new_category.hub_id, 9);
                assert_eq!(new_category.name, "Fresh Produce");
                assert_eq!(new_category.parent_id, Some(4));
                true
            })
            .returning(|_| Ok(sample_category(10, 9, "Fresh Produce")));

        let form = AddCategoryForm {
            name: "  Fresh   Produce ".to_string(),
            description: Some(" seasonal goods ".to_string()),
            parent_id: Some("4".to_string()),
        };

        let created = create_category(&repo, &user, form).expect("expected success");

        assert_eq!(created.id, 10);
        assert_eq!(created.name, "Fresh Produce");
    }

    #[test]
    fn assign_child_categories_requires_role() {
        let repo = MockCategoryWriter::new();
        let user = user_with_roles(&[]);
        let form = AssignChildCategoriesForm {
            parent_id: 5,
            child_ids: vec![6, 7],
        };

        let result = assign_child_categories(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn assign_child_categories_filters_invalid_ids() {
        let mut repo = MockCategoryWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_assign_child_categories()
            .times(1)
            .withf(|hub_id, parent_id, child_ids| {
                assert_eq!(*hub_id, 9);
                assert_eq!(*parent_id, 5);
                assert_eq!(child_ids, &[6, 8]);
                true
            })
            .returning(|_, _, _| Ok(sample_category(5, 9, "Parent")));

        let form = AssignChildCategoriesForm {
            parent_id: 5,
            child_ids: vec![6, 8, 5, 6, -1],
        };

        let result = assign_child_categories(&repo, &user, form);

        assert!(result.is_ok());
    }

    #[test]
    fn modify_category_requires_role() {
        let repo = MockCategoryRepo::new();
        let user = user_with_roles(&[]);
        let form = EditCategoryForm {
            category_id: 1,
            name: "Updated".to_string(),
            description: None,
            parent_id: None,
            is_archived: None,
        };

        let result = modify_category(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn modify_category_rejects_self_parent() {
        let repo = MockCategoryRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditCategoryForm {
            category_id: 3,
            name: "Pantry".to_string(),
            description: None,
            parent_id: Some("3".to_string()),
            is_archived: None,
        };

        let result = modify_category(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn modify_category_rejects_descendant_parent() {
        let mut repo = MockCategoryRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.reader
            .expect_list_categories()
            .times(1)
            .withf(|query| {
                assert_eq!(query.hub_id, 9);
                assert!(query.include_archived);
                true
            })
            .returning(|_| {
                Ok((
                    3,
                    vec![
                        sample_category(1, 9, "Root"),
                        Category {
                            id: 2,
                            hub_id: 9,
                            parent_id: Some(1),
                            name: "Child".to_string(),
                            description: None,
                            is_archived: false,
                            created_at: fixed_datetime(),
                            updated_at: fixed_datetime(),
                        },
                        Category {
                            id: 3,
                            hub_id: 9,
                            parent_id: Some(2),
                            name: "Grandchild".to_string(),
                            description: None,
                            is_archived: false,
                            created_at: fixed_datetime(),
                            updated_at: fixed_datetime(),
                        },
                    ],
                ))
            });

        let form = EditCategoryForm {
            category_id: 1,
            name: "Root".to_string(),
            description: None,
            parent_id: Some("3".to_string()),
            is_archived: None,
        };

        let result = modify_category(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn modify_category_updates_entry() {
        let mut repo = MockCategoryRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.writer
            .expect_update_category()
            .times(1)
            .withf(|category_id, hub_id, updates| {
                assert_eq!(*category_id, 3);
                assert_eq!(*hub_id, 9);
                assert_eq!(updates.name, "Dry Goods");
                assert_eq!(updates.description.as_deref(), Some("pantry items"));
                assert!(updates.parent_id.is_none());
                true
            })
            .returning(|_, _, _| Ok(sample_category(3, 9, "Dry Goods")));

        let form = EditCategoryForm {
            category_id: 3,
            name: " Dry Goods ".to_string(),
            description: Some(" pantry items ".to_string()),
            parent_id: Some("".to_string()),
            is_archived: Some(false),
        };

        let updated = modify_category(&repo, &user, form).expect("expected success");

        assert_eq!(updated.id, 3);
    }

    #[test]
    fn remove_category_requires_role() {
        let repo = MockCategoryWriter::new();
        let user = user_with_roles(&[]);

        let result = remove_category(&repo, &user, 2);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn remove_category_deletes_entry() {
        let mut repo = MockCategoryWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_category()
            .times(1)
            .withf(|category_id, hub_id| {
                assert_eq!(*category_id, 2);
                assert_eq!(*hub_id, 9);
                true
            })
            .returning(|_, _| Ok(()));

        let result = remove_category(&repo, &user, 2);

        assert!(result.is_ok());
    }
}
