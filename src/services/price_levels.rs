use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::Deserialize;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::price_level::{PriceLevel, PriceLevelListQuery};
use crate::repository::PriceLevelReader;
use crate::services::{ServiceError, ServiceResult};

/// Query parameters accepted by the price levels index page.
#[derive(Debug, Default, Deserialize)]
pub struct PriceLevelsQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
}

/// Data required to render the price levels index template.
pub struct PriceLevelsPageData {
    /// Paginated list of price levels to show in the table.
    pub price_levels: Paginated<PriceLevel>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}

/// Loads the price levels list for the index page.
pub fn load_price_levels<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: PriceLevelsQuery,
) -> ServiceResult<PriceLevelsPageData>
where
    R: PriceLevelReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let page = query.page.unwrap_or(1);
    let mut list_query = PriceLevelListQuery::new(user.hub_id);

    if let Some(value) = query.search.as_ref() {
        list_query = list_query.search(value);
    }

    list_query = list_query.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let (total, price_levels) = repo
        .list_price_levels(list_query)
        .map_err(ServiceError::from)?;

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let price_levels = Paginated::new(price_levels, page, total_pages);

    Ok(PriceLevelsPageData {
        price_levels,
        search: query.search,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::Value;

    use crate::domain::price_level::PriceLevel;
    use crate::repository::mock::MockPriceLevelReader;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_level(id: i32, hub_id: i32, name: &str) -> PriceLevel {
        PriceLevel {
            id,
            hub_id,
            name: name.to_string(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 42,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    #[test]
    fn load_price_levels_returns_unauthorized_when_role_missing() {
        let repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[]);

        let result = load_price_levels(&repo, &user, PriceLevelsQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_price_levels_returns_paginated_data() {
        let mut repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = PriceLevelsQuery {
            search: Some("sil".to_string()),
            page: Some(2),
        };

        let expected_hub = user.hub_id;
        let expected_per_page = DEFAULT_ITEMS_PER_PAGE;

        repo.expect_list_price_levels()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id, expected_hub);
                assert_eq!(query.search.as_deref(), Some("sil"));
                match &query.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 2);
                        assert_eq!(pagination.per_page, expected_per_page);
                    }
                    None => panic!("expected pagination to be set"),
                }
                true
            })
            .returning(move |_| {
                Ok((
                    5,
                    vec![
                        sample_level(1, expected_hub, "Silver"),
                        sample_level(2, expected_hub, "Gold"),
                    ],
                ))
            });

        let result = load_price_levels(&repo, &user, query);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.search.as_deref(), Some("sil"));

        let serialized = match serde_json::to_value(&data.price_levels) {
            Ok(value) => value,
            Err(err) => panic!("serialization failed: {err}"),
        };

        let page_value = match serialized.get("page") {
            Some(value) => value,
            None => panic!("missing page field"),
        };
        assert_eq!(page_value.as_u64(), Some(2));

        let items = match serialized.get("items") {
            Some(value) => match value.as_array() {
                Some(items) => items,
                None => panic!("items field is not an array"),
            },
            None => panic!("missing items field"),
        };
        assert_eq!(items.len(), 2);

        let first_name = items
            .first()
            .and_then(Value::as_object)
            .and_then(|map| map.get("name"))
            .and_then(Value::as_str);
        assert_eq!(first_name, Some("Silver"));
    }
}
