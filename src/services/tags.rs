use chrono::Utc;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::Deserialize;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::tag::{Tag, TagListQuery};
use crate::forms::tags::{AddTagForm, EditTagForm};
use crate::repository::{TagReader, TagWriter};
use crate::services::{ServiceError, ServiceResult};

/// Query parameters accepted by the tags index page.
#[derive(Debug, Default, Deserialize)]
pub struct TagQuery {
    /// Optional case-insensitive search applied to tag names.
    pub search: Option<String>,
    /// Page number requested by the UI (1-based).
    pub page: Option<usize>,
}

/// Data required to render the tags index template.
pub struct TagsPageData {
    /// Paginated list of tags displayed in the table.
    pub tags: Paginated<Tag>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}

/// Fetches paginated tags for the authenticated user's hub.
pub fn load_tags<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: TagQuery,
) -> ServiceResult<TagsPageData>
where
    R: TagReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let TagQuery { search, page } = query;
    let page = page.unwrap_or(1);

    let mut list_query = TagListQuery::new(user.hub_id);

    if let Some(term) = search.as_ref() {
        list_query = list_query.search(term);
    }

    list_query = list_query.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let (total, tags) = repo.list_tags(list_query).map_err(ServiceError::from)?;
    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let tags = Paginated::new(tags, page, total_pages);

    Ok(TagsPageData { tags, search })
}

/// Creates a new tag for the authenticated user's hub.
pub fn create_tag<R>(repo: &R, user: &AuthenticatedUser, form: AddTagForm) -> ServiceResult<Tag>
where
    R: TagWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let new_tag = form
        .into_new_tag(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_tag(&new_tag).map_err(ServiceError::from)
}

/// Updates an existing tag for the authenticated user's hub.
pub fn modify_tag<R>(repo: &R, user: &AuthenticatedUser, form: EditTagForm) -> ServiceResult<Tag>
where
    R: TagWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let tag_id = form.tag_id;
    let update = form
        .into_update_tag(Utc::now().naive_utc())
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.update_tag(tag_id, user.hub_id, &update)
        .map_err(ServiceError::from)
}

/// Deletes a tag for the authenticated user's hub.
pub fn remove_tag<R>(repo: &R, user: &AuthenticatedUser, tag_id: i32) -> ServiceResult<()>
where
    R: TagWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    repo.delete_tag(tag_id, user.hub_id)
        .map_err(ServiceError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::Value;

    use crate::repository::mock::{MockTagReader, MockTagWriter};

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 7,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    fn sample_tag(id: i32, hub_id: i32, name: &str) -> Tag {
        Tag {
            id,
            hub_id,
            name: name.to_string(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

    #[test]
    fn load_tags_rejects_missing_role() {
        let repo = MockTagReader::new();
        let user = user_with_roles(&[]);

        let result = load_tags(&repo, &user, TagQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_tags_returns_paginated_data() {
        let mut repo = MockTagReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = TagQuery {
            search: Some("sea".to_string()),
            page: Some(2),
        };
        let expected_hub = user.hub_id;

        repo.expect_list_tags()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id, expected_hub);
                assert_eq!(query.search.as_deref(), Some("sea"));
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
                    45,
                    vec![
                        sample_tag(1, expected_hub, "Seasonal"),
                        sample_tag(2, expected_hub, "Seaside"),
                    ],
                ))
            });

        let result = load_tags(&repo, &user, query);
        let data = result.expect("expected success");

        assert_eq!(data.search.as_deref(), Some("sea"));

        let serialized = serde_json::to_value(&data.tags).expect("serialization should succeed");

        let page_value = serialized
            .get("page")
            .and_then(Value::as_u64)
            .expect("expected page field");
        assert_eq!(page_value, 2);

        let items = serialized
            .get("items")
            .and_then(Value::as_array)
            .expect("expected items array");
        assert_eq!(items.len(), 2);

        let first_name = items
            .first()
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("name"))
            .and_then(Value::as_str);
        assert_eq!(first_name, Some("Seasonal"));

        let pages = serialized
            .get("pages")
            .and_then(Value::as_array)
            .expect("expected pages array");
        let last_page = pages.iter().rev().find_map(|value| value.as_u64());
        const TOTAL: usize = 45;
        let expected_pages = if TOTAL == 0 {
            0
        } else {
            TOTAL.div_ceil(DEFAULT_ITEMS_PER_PAGE)
        };

        assert_eq!(last_page.map(|value| value as usize), Some(expected_pages));
    }

    #[test]
    fn create_tag_requires_role() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[]);
        let form = AddTagForm {
            name: "Retail".to_string(),
        };

        let result = create_tag(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn create_tag_validates_and_persists() {
        let mut repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_create_tag()
            .times(1)
            .withf(|new_tag| {
                assert_eq!(new_tag.hub_id, 7);
                assert_eq!(new_tag.name, "Seasonal Picks");
                true
            })
            .returning(|_| Ok(sample_tag(3, 7, "Seasonal Picks")));

        let form = AddTagForm {
            name: "  Seasonal\tPicks  ".to_string(),
        };

        let created = create_tag(&repo, &user, form).expect("expected success");

        assert_eq!(created.id, 3);
        assert_eq!(created.name, "Seasonal Picks");
    }

    #[test]
    fn create_tag_returns_form_error() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTagForm {
            name: "   ".to_string(),
        };

        let result = create_tag(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn modify_tag_requires_role() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[]);
        let form = EditTagForm {
            tag_id: 1,
            name: "Updated".to_string(),
        };

        let result = modify_tag(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn modify_tag_updates_repository() {
        let mut repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_update_tag()
            .times(1)
            .withf(|tag_id, hub_id, updates| {
                assert_eq!(*tag_id, 5);
                assert_eq!(*hub_id, 7);
                assert_eq!(updates.name, "Limited Edition");
                true
            })
            .returning(|_, _, _| Ok(sample_tag(5, 7, "Limited Edition")));

        let form = EditTagForm {
            tag_id: 5,
            name: "  Limited\nEdition  ".to_string(),
        };

        let updated = modify_tag(&repo, &user, form).expect("expected success");

        assert_eq!(updated.id, 5);
        assert_eq!(updated.name, "Limited Edition");
    }

    #[test]
    fn modify_tag_returns_form_error() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditTagForm {
            tag_id: 5,
            name: "   ".to_string(),
        };

        let result = modify_tag(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn remove_tag_requires_role() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[]);

        let result = remove_tag(&repo, &user, 1);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn remove_tag_deletes_record() {
        let mut repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_tag()
            .times(1)
            .withf(|tag_id, hub_id| {
                assert_eq!(*tag_id, 4);
                assert_eq!(*hub_id, 7);
                true
            })
            .returning(|_, _| Ok(()));

        let result = remove_tag(&repo, &user, 4);

        assert!(matches!(result, Ok(())));
    }
}
