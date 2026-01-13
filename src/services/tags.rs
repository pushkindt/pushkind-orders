use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::ensure_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::tag::{NewTag, Tag, TagListQuery, UpdateTag};
use crate::domain::types::{HubId, TagId};
use crate::dto::tags::{TagQuery, TagsPageData};
use crate::forms::tags::{AddTagForm, AddTagPayload, EditTagForm, EditTagPayload};
use crate::repository::{TagReader, TagWriter};
use crate::services::{ServiceError, ServiceResult};

/// Fetches paginated tags for the authenticated user's hub.
pub fn load_tags<R>(
    query: TagQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<TagsPageData>
where
    R: TagReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let TagQuery { search, page } = query;
    let page = page.unwrap_or(1);

    let mut list_query = TagListQuery::try_new(user.hub_id)?;

    if let Some(term) = search.as_ref() {
        list_query = list_query.search(term);
    }

    list_query = list_query.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let (total, tags) = repo.list_tags(list_query)?;
    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let tags = Paginated::new(tags, page, total_pages);

    Ok(TagsPageData { tags, search })
}

/// Fetches a single tag for the authenticated user's hub.
pub fn load_tag_for_edit<R>(tag_id: i32, user: &AuthenticatedUser, repo: &R) -> ServiceResult<Tag>
where
    R: TagReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;
    let tag_id = TagId::new(tag_id)?;

    match repo.get_tag_by_id(tag_id, hub_id)? {
        Some(tag) => Ok(tag),
        None => Err(ServiceError::NotFound),
    }
}

/// Creates a new tag for the authenticated user's hub.
pub fn create_tag<R>(form: AddTagForm, user: &AuthenticatedUser, repo: &R) -> ServiceResult<Tag>
where
    R: TagWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let payload: AddTagPayload = form.try_into()?;
    let hub_id = HubId::new(user.hub_id)?;
    let new_tag = NewTag::new(hub_id, payload.name);

    Ok(repo.create_tag(&new_tag)?)
}

/// Updates an existing tag for the authenticated user's hub.
pub fn modify_tag<R>(form: EditTagForm, user: &AuthenticatedUser, repo: &R) -> ServiceResult<Tag>
where
    R: TagWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let payload: EditTagPayload = form.try_into()?;
    let hub_id = HubId::new(user.hub_id)?;
    let update = UpdateTag::new(payload.name);

    Ok(repo.update_tag(payload.tag_id, hub_id, &update)?)
}

/// Deletes a tag for the authenticated user's hub.
pub fn remove_tag<R>(tag_id: i32, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: TagWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let tag_id = TagId::new(tag_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    Ok(repo.delete_tag(tag_id, hub_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::Value;

    use crate::domain::types::{HubId, TagId, TagName};
    use crate::dto::tags::TagQuery;
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
            id: TagId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: TagName::new(name).unwrap(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

    #[test]
    fn load_tags_rejects_missing_role() {
        let repo = MockTagReader::new();
        let user = user_with_roles(&[]);

        let result = load_tags(TagQuery::default(), &user, &repo);

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
        let expected_hub_id = HubId::new(expected_hub).unwrap();

        repo.expect_list_tags()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id, expected_hub_id);
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

        let result = load_tags(query, &user, &repo);
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

        let result = create_tag(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_tag_for_edit_requires_role() {
        let repo = MockTagReader::new();
        let user = user_with_roles(&[]);

        let result = load_tag_for_edit(3, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_tag_for_edit_returns_not_found() {
        let mut repo = MockTagReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_get_tag_by_id()
            .times(1)
            .withf(|tag_id, hub_id| {
                assert_eq!(tag_id.get(), 9);
                assert_eq!(hub_id.get(), 7);
                true
            })
            .returning(|_, _| Ok(None));

        let result = load_tag_for_edit(9, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_tag_for_edit_returns_tag() {
        let mut repo = MockTagReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_get_tag_by_id()
            .times(1)
            .withf(|tag_id, hub_id| {
                assert_eq!(tag_id.get(), 12);
                assert_eq!(hub_id.get(), 7);
                true
            })
            .returning(|_, _| Ok(Some(sample_tag(12, 7, "Signature"))));

        let result = load_tag_for_edit(12, &user, &repo).expect("expected tag");

        assert_eq!(result.id.get(), 12);
        assert_eq!(result.name.as_str(), "Signature");
    }

    #[test]
    fn create_tag_validates_and_persists() {
        let mut repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_create_tag()
            .times(1)
            .withf(|new_tag| {
                assert_eq!(new_tag.hub_id.get(), 7);
                assert_eq!(new_tag.name.as_str(), "Seasonal\tPicks");
                true
            })
            .returning(|_| Ok(sample_tag(3, 7, "Seasonal\tPicks")));

        let form = AddTagForm {
            name: "  Seasonal\tPicks  ".to_string(),
        };

        let created = create_tag(form, &user, &repo).expect("expected success");

        assert_eq!(created.id.get(), 3);
        assert_eq!(created.name.as_str(), "Seasonal\tPicks");
    }

    #[test]
    fn create_tag_returns_form_error() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTagForm {
            name: "   ".to_string(),
        };

        let result = create_tag(form, &user, &repo);

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

        let result = modify_tag(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn modify_tag_updates_repository() {
        let mut repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_update_tag()
            .times(1)
            .withf(|tag_id, hub_id, updates| {
                assert_eq!(tag_id.get(), 5);
                assert_eq!(hub_id.get(), 7);
                assert_eq!(updates.name.as_str(), "Limited\nEdition");
                true
            })
            .returning(|_, _, _| Ok(sample_tag(5, 7, "Limited\nEdition")));

        let form = EditTagForm {
            tag_id: 5,
            name: "  Limited\nEdition  ".to_string(),
        };

        let updated = modify_tag(form, &user, &repo).expect("expected success");

        assert_eq!(updated.id.get(), 5);
        assert_eq!(updated.name.as_str(), "Limited\nEdition");
    }

    #[test]
    fn modify_tag_returns_form_error() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditTagForm {
            tag_id: 5,
            name: "   ".to_string(),
        };

        let result = modify_tag(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn remove_tag_requires_role() {
        let repo = MockTagWriter::new();
        let user = user_with_roles(&[]);

        let result = remove_tag(1, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn remove_tag_deletes_record() {
        let mut repo = MockTagWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_tag()
            .times(1)
            .withf(|tag_id, hub_id| {
                assert_eq!(tag_id.get(), 4);
                assert_eq!(hub_id.get(), 7);
                true
            })
            .returning(|_, _| Ok(()));

        let result = remove_tag(4, &user, &repo);

        assert!(matches!(result, Ok(())));
    }
}
