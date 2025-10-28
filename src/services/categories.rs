use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::category::{Category, CategoryTreeNode, CategoryTreeQuery};
use crate::forms::categories::{AddCategoryForm, EditCategoryForm};
use crate::repository::{CategoryReader, CategoryWriter};
use crate::services::{ServiceError, ServiceResult};

/// Data required to render the categories index template.
pub struct CategoryTreeData {
    /// Hierarchical representation of the categories.
    pub tree: Vec<CategoryTreeNode>,
}

/// Loads the categories overview page.
pub fn load_categories<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<CategoryTreeData>
where
    R: CategoryReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let (_, mut flat) = repo
        .list_categories(CategoryTreeQuery::new(user.hub_id).include_archived())
        .map_err(ServiceError::from)?;

    if flat.is_empty() {
        return Ok(CategoryTreeData { tree: Vec::new() });
    }

    flat.sort_by(|a, b| a.name.cmp(&b.name));
    let tree = build_category_tree(&flat);

    Ok(CategoryTreeData { tree })
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

    let category_id = form.category_id;

    let update = form
        .into_update_category()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.update_category(category_id, user.hub_id, &update)
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

fn build_category_tree(categories: &[Category]) -> Vec<CategoryTreeNode> {
    let mut children_by_parent: HashMap<Option<i32>, Vec<&Category>> = HashMap::new();

    for category in categories {
        children_by_parent
            .entry(category.parent_id)
            .or_default()
            .push(category);
    }

    for children in children_by_parent.values_mut() {
        children.sort_by(|a, b| a.name.cmp(&b.name));
    }

    fn build_branch(
        parent_id: Option<i32>,
        grouped: &HashMap<Option<i32>, Vec<&Category>>,
    ) -> Vec<CategoryTreeNode> {
        match grouped.get(&parent_id) {
            Some(children) => {
                let mut nodes = Vec::with_capacity(children.len());
                for category in children {
                    let sub_tree = build_branch(Some(category.id), grouped);
                    nodes.push(CategoryTreeNode::new((*category).clone()).with_children(sub_tree));
                }
                nodes
            }
            None => Vec::new(),
        }
    }

    build_branch(None, &children_by_parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::domain::category::{
        NewCategory as DomainNewCategory, UpdateCategory as DomainUpdateCategory,
    };
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
        fn list_categories(
            &self,
            query: CategoryTreeQuery,
        ) -> RepositoryResult<(usize, Vec<Category>)> {
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
            self.writer
                .assign_child_categories(hub_id, parent_id, child_ids)
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

        let result = load_categories(&repo, &user);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_categories_returns_category_tree() {
        let mut repo = MockCategoryReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let expected_hub = user.hub_id;

        repo.expect_list_categories()
            .times(1)
            .returning(move |query| {
                assert_eq!(query.hub_id, expected_hub);
                assert!(query.include_archived);
                assert!(query.search.is_none());
                assert!(query.pagination.is_none());

                let beverages = sample_category(1, expected_hub, "Beverages");
                let mut hot_drinks = sample_category(2, expected_hub, "Hot Drinks");
                hot_drinks.parent_id = Some(beverages.id);
                hot_drinks.is_archived = true;
                let mut coffee = sample_category(3, expected_hub, "Coffee");
                coffee.parent_id = Some(hot_drinks.id);

                Ok((3, vec![beverages, hot_drinks, coffee]))
            });

        let data = load_categories(&repo, &user).expect("expected success");

        assert_eq!(data.tree.len(), 1);
        let root = &data.tree[0];
        assert_eq!(root.category.name, "Beverages");
        assert_eq!(root.children.len(), 1);

        let child = &root.children[0];
        assert_eq!(child.category.name, "Hot Drinks");
        assert!(child.category.is_archived);
        assert_eq!(child.children.len(), 1);
        assert_eq!(child.children[0].category.name, "Coffee");
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
    fn modify_category_requires_role() {
        let repo = MockCategoryRepo::new();
        let user = user_with_roles(&[]);
        let form = EditCategoryForm {
            category_id: 1,
            name: "Updated".to_string(),
            description: None,
            is_archived: false,
        };

        let result = modify_category(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
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
                true
            })
            .returning(|_, _, _| Ok(sample_category(3, 9, "Dry Goods")));

        let form = EditCategoryForm {
            category_id: 3,
            name: " Dry Goods ".to_string(),
            description: Some(" pantry items ".to_string()),
            is_archived: false,
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
