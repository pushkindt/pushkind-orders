use std::collections::HashSet;

use pushkind_common::domain::auth::AuthenticatedUser;

use crate::domain::category::CategoryTreeQuery;
use crate::domain::customer::{CustomerListQuery, NewCustomer, UpdateCustomer};
use crate::domain::price_level::{
    NewPriceLevel, PriceLevel, PriceLevelListQuery, UpdatePriceLevel,
};
use crate::domain::product::ProductListQuery;
use crate::domain::product_price_level::NewProductPriceLevelRate;
use crate::domain::types::{HubId, PriceCents, PriceLevelId};
use crate::dto::price_levels::{
    ClientPriceLevelAssignment, ClientPriceLevelAssignments, PriceLevelsPageData, PriceLevelsQuery,
};
use crate::forms::price_levels::{
    AddPriceLevelForm, AddPriceLevelPayload, AssignClientPriceLevelForm,
    AssignClientPriceLevelPayload, EditPriceLevelForm, EditPriceLevelPayload, PriceModifierKind,
};
use crate::repository::{
    CategoryReader, CustomerReader, CustomerWriter, PriceLevelReader, PriceLevelWriter,
    ProductReader, ProductWriter,
};
use crate::services::{ServiceError, ServiceResult, ensure_admin, ensure_catalog_read_access};

/// Loads the price levels list for the index page.
pub fn load_price_levels<R>(
    query: PriceLevelsQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<PriceLevelsPageData>
where
    R: PriceLevelReader + CategoryReader + ?Sized,
{
    ensure_catalog_read_access(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let mut list_query = PriceLevelListQuery::new(hub_id);

    if let Some(value) = query.search.as_ref() {
        list_query = list_query.search(value);
    }

    let (_total, price_levels) = repo.list_price_levels(list_query)?;

    let (_, mut categories) = repo.list_categories(CategoryTreeQuery::new(hub_id))?;
    categories.retain(|category| !category.is_archived);
    categories.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));

    Ok(PriceLevelsPageData {
        price_levels,
        search: query.search,
        categories,
    })
}

/// Loads a single price level for the authenticated user's hub.
pub fn load_price_level_for_edit<R>(
    price_level_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<PriceLevel>
where
    R: PriceLevelReader + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let price_level_id = PriceLevelId::new(price_level_id)?;

    match repo.get_price_level_by_id(price_level_id, hub_id)? {
        Some(price_level) => Ok(price_level),
        None => Err(ServiceError::NotFound),
    }
}

/// Loads saved price level assignments for all hub customers.
pub fn load_client_price_level_assignments<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientPriceLevelAssignments>
where
    R: PriceLevelReader + CustomerReader + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let (_, price_levels) = repo.list_price_levels(PriceLevelListQuery::new(hub_id))?;

    let default_price_level_id = price_levels
        .iter()
        .find(|level| level.is_default)
        .map(|level| level.id.get());

    let (_, customers) = repo.list_customers(CustomerListQuery::new(hub_id))?;

    let assignments = customers
        .into_iter()
        .map(ClientPriceLevelAssignment::from)
        .collect();

    Ok(ClientPriceLevelAssignments {
        hub_id: user.hub_id,
        default_price_level_id,
        assignments,
    })
}

/// Creates a new price level for the authenticated user's hub.
pub fn create_price_level<R>(
    form: AddPriceLevelForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<PriceLevel>
where
    R: PriceLevelWriter + ProductReader + ProductWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let payload: AddPriceLevelPayload = form.try_into()?;
    let new_price_level = NewPriceLevel::new(hub_id, payload.name, payload.default);
    let modifier_input = payload.modifier_input;

    let (_, mut products) = repo.list_products(ProductListQuery::new(hub_id))?;
    let included_products: HashSet<_> = modifier_input
        .included_product_ids
        .iter()
        .cloned()
        .collect();

    if !modifier_input.excluded_category_ids.is_empty() {
        let excluded: HashSet<_> = modifier_input.excluded_category_ids.into_iter().collect();
        products.retain(|product| {
            if included_products.contains(&product.id) {
                return true;
            }
            product
                .category_id
                .map(|category_id| !excluded.contains(&category_id))
                .unwrap_or(true)
        });
    }

    if !modifier_input.excluded_product_ids.is_empty() {
        let excluded: HashSet<_> = modifier_input.excluded_product_ids.into_iter().collect();
        products.retain(|product| {
            included_products.contains(&product.id) || !excluded.contains(&product.id)
        });
    }

    let mut seed_rates = Vec::new();
    for product in products {
        let base_rate = product
            .price_levels
            .iter()
            .find(|rate| rate.price_level_id == modifier_input.base_price_level_id);

        let Some(base_rate) = base_rate else {
            continue;
        };

        let adjusted = apply_price_modifier(
            base_rate.price_cents,
            modifier_input.price_modifier,
            modifier_input.price_modifier_kind,
        )?;

        seed_rates.push((product.id, adjusted));
    }

    let created = repo.create_price_level(&new_price_level)?;

    if !seed_rates.is_empty() {
        let rates: Vec<NewProductPriceLevelRate> = seed_rates
            .into_iter()
            .map(|(product_id, price_cents)| {
                NewProductPriceLevelRate::new(product_id, created.id, price_cents)
            })
            .collect();

        if let Err(err) = repo.create_product_price_levels(hub_id, &rates) {
            let _ = repo.delete_price_level(created.id, hub_id);
            return Err(ServiceError::from(err));
        }
    }

    Ok(created)
}

fn apply_price_modifier(
    base_price: PriceCents,
    modifier: i32,
    modifier_kind: PriceModifierKind,
) -> ServiceResult<PriceCents> {
    let base = i64::from(base_price.get());
    let adjusted = match modifier_kind {
        PriceModifierKind::Percent => base * i64::from(100 + modifier) / 100,
        PriceModifierKind::Fixed => base + i64::from(modifier),
    };

    let adjusted = i32::try_from(adjusted).map_err(|_| ServiceError::Internal)?;
    PriceCents::new(adjusted)
        .map_err(|_| ServiceError::Form("price modifier results in non-positive price".to_string()))
}

/// Updates an existing price level for the authenticated user's hub.
pub fn update_price_level<R>(
    price_level_id: i32,
    form: EditPriceLevelForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<PriceLevel>
where
    R: PriceLevelWriter + ?Sized,
{
    ensure_admin(user)?;

    let payload: EditPriceLevelPayload = form.try_into()?;
    let updates = UpdatePriceLevel::new(payload.name, payload.default);

    let hub_id = HubId::new(user.hub_id)?;
    let price_level_id = PriceLevelId::new(price_level_id)?;

    Ok(repo.update_price_level(price_level_id, hub_id, &updates)?)
}

/// Deletes a price level for the authenticated user's hub.
pub fn remove_price_level<R>(
    price_level_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: PriceLevelWriter + ?Sized,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let price_level_id = PriceLevelId::new(price_level_id)?;

    Ok(repo.delete_price_level(price_level_id, hub_id)?)
}

/// Persists a price level assignment for a single customer.
pub fn assign_price_level_to_client<R>(
    payload: AssignClientPriceLevelForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: CustomerReader + CustomerWriter + ?Sized,
{
    ensure_admin(user)?;

    let assignment: AssignClientPriceLevelPayload = payload.try_into()?;

    let hub_id = HubId::new(user.hub_id)?;

    match repo.get_customer_by_phone(&assignment.phone, hub_id)? {
        Some(existing) => {
            let updates = UpdateCustomer {
                name: assignment.name,
                public_id: Some(assignment.public_id),
                price_level_id: assignment.price_level_id,
            };

            repo.update_customer(existing.id, hub_id, &updates)?;
            Ok(())
        }
        None => {
            let mut new_customer = NewCustomer::try_new(
                user.hub_id,
                assignment.name.clone(),
                assignment.phone.clone(),
            )?;

            new_customer = new_customer.try_with_public_id(assignment.public_id)?;

            if let Some(price_level_id) = assignment.price_level_id {
                new_customer = new_customer.with_price_level_id(price_level_id);
            }

            repo.create_customer(&new_customer)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::SERVICE_ACCESS_ROLE;
    use crate::domain::category::{Category, CategoryTreeQuery};
    use crate::domain::customer::{Customer, CustomerListQuery, NewCustomer, UpdateCustomer};
    use crate::domain::price_level::PriceLevel;
    use crate::domain::product::{Product, ProductListQuery};
    use crate::domain::product_price_level::ProductPriceLevelRate;
    use crate::domain::types::{
        CategoryId, CategoryName, CurrencyCode, CustomerId, CustomerName, HubId, PhoneNumber,
        PriceCents, PriceLevelId, PriceLevelName, ProductId, ProductName, ProductPriceLevelRateId,
    };
    use crate::dto::price_levels::{ClientPriceLevelAssignment, PriceLevelsQuery};
    use crate::forms::price_levels::{
        AddPriceLevelForm, AssignClientPriceLevelForm, PriceModifierKind,
    };
    use crate::repository::mock::{
        MockCategoryReader, MockCustomerReader, MockCustomerWriter, MockPriceLevelReader,
        MockPriceLevelWriter, MockProductReader, MockProductWriter,
    };
    use crate::repository::{
        CategoryReader, CustomerReader, CustomerWriter, PriceLevelReader, ProductReader,
        ProductWriter,
    };
    use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

    struct CombinedCustomerRepo {
        reader: MockCustomerReader,
        writer: MockCustomerWriter,
    }

    impl CombinedCustomerRepo {
        fn new(reader: MockCustomerReader, writer: MockCustomerWriter) -> Self {
            Self { reader, writer }
        }
    }

    impl CustomerReader for CombinedCustomerRepo {
        fn get_customer_by_id(
            &self,
            id: CustomerId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.reader.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_phone(
            &self,
            phone: &PhoneNumber,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.reader.get_customer_by_phone(phone, hub_id)
        }

        fn list_customers(
            &self,
            query: CustomerListQuery,
        ) -> RepositoryResult<(usize, Vec<Customer>)> {
            self.reader.list_customers(query)
        }
    }

    impl CustomerWriter for CombinedCustomerRepo {
        fn create_customer(&self, new_customer: &NewCustomer) -> RepositoryResult<Customer> {
            self.writer.create_customer(new_customer)
        }

        fn assign_price_level_to_customers(
            &self,
            hub_id: HubId,
            customer_ids: &[CustomerId],
            price_level_id: Option<PriceLevelId>,
        ) -> RepositoryResult<()> {
            self.writer
                .assign_price_level_to_customers(hub_id, customer_ids, price_level_id)
        }

        fn update_customer(
            &self,
            customer_id: CustomerId,
            hub_id: HubId,
            updates: &UpdateCustomer,
        ) -> RepositoryResult<Customer> {
            self.writer.update_customer(customer_id, hub_id, updates)
        }
    }

    struct CombinedPriceLevelRepo {
        price_reader: MockPriceLevelReader,
        category_reader: MockCategoryReader,
    }

    impl CombinedPriceLevelRepo {
        fn new(price_reader: MockPriceLevelReader, category_reader: MockCategoryReader) -> Self {
            Self {
                price_reader,
                category_reader,
            }
        }
    }

    impl PriceLevelReader for CombinedPriceLevelRepo {
        fn get_price_level_by_id(
            &self,
            id: PriceLevelId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<PriceLevel>> {
            self.price_reader.get_price_level_by_id(id, hub_id)
        }

        fn list_price_levels(
            &self,
            query: PriceLevelListQuery,
        ) -> RepositoryResult<(usize, Vec<PriceLevel>)> {
            self.price_reader.list_price_levels(query)
        }
    }

    impl CategoryReader for CombinedPriceLevelRepo {
        fn list_categories(
            &self,
            query: CategoryTreeQuery,
        ) -> RepositoryResult<(usize, Vec<Category>)> {
            self.category_reader.list_categories(query)
        }

        fn get_category_by_id(
            &self,
            category_id: CategoryId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Category>> {
            self.category_reader.get_category_by_id(category_id, hub_id)
        }

        fn get_category_by_name_and_parent(
            &self,
            name: &CategoryName,
            parent_id: Option<CategoryId>,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Category>> {
            self.category_reader
                .get_category_by_name_and_parent(name, parent_id, hub_id)
        }
    }

    struct CombinedPriceLevelCreateRepo {
        price_writer: MockPriceLevelWriter,
        product_reader: MockProductReader,
        product_writer: MockProductWriter,
    }

    impl CombinedPriceLevelCreateRepo {
        fn new(
            price_writer: MockPriceLevelWriter,
            product_reader: MockProductReader,
            product_writer: MockProductWriter,
        ) -> Self {
            Self {
                price_writer,
                product_reader,
                product_writer,
            }
        }
    }

    impl PriceLevelWriter for CombinedPriceLevelCreateRepo {
        fn create_price_level(
            &self,
            new_price_level: &crate::domain::price_level::NewPriceLevel,
        ) -> RepositoryResult<PriceLevel> {
            self.price_writer.create_price_level(new_price_level)
        }

        fn update_price_level(
            &self,
            price_level_id: PriceLevelId,
            hub_id: HubId,
            updates: &crate::domain::price_level::UpdatePriceLevel,
        ) -> RepositoryResult<PriceLevel> {
            self.price_writer
                .update_price_level(price_level_id, hub_id, updates)
        }

        fn delete_price_level(
            &self,
            price_level_id: PriceLevelId,
            hub_id: HubId,
        ) -> RepositoryResult<()> {
            self.price_writer.delete_price_level(price_level_id, hub_id)
        }
    }

    impl ProductReader for CombinedPriceLevelCreateRepo {
        fn get_product_by_id(
            &self,
            id: ProductId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Product>> {
            self.product_reader.get_product_by_id(id, hub_id)
        }

        fn list_products(
            &self,
            query: ProductListQuery,
        ) -> RepositoryResult<(usize, Vec<Product>)> {
            self.product_reader.list_products(query)
        }
    }

    impl ProductWriter for CombinedPriceLevelCreateRepo {
        fn create_product(
            &self,
            new_product: &crate::domain::product::NewProduct,
        ) -> RepositoryResult<Product> {
            self.product_writer.create_product(new_product)
        }

        fn update_product(
            &self,
            product_id: ProductId,
            hub_id: HubId,
            updates: &crate::domain::product::UpdateProduct,
        ) -> RepositoryResult<Product> {
            self.product_writer
                .update_product(product_id, hub_id, updates)
        }

        fn delete_product(&self, product_id: ProductId, hub_id: HubId) -> RepositoryResult<()> {
            self.product_writer.delete_product(product_id, hub_id)
        }

        fn replace_product_price_levels(
            &self,
            product_id: ProductId,
            hub_id: HubId,
            rates: &[crate::domain::product_price_level::NewProductPriceLevelRate],
        ) -> RepositoryResult<()> {
            self.product_writer
                .replace_product_price_levels(product_id, hub_id, rates)
        }

        fn create_product_price_levels(
            &self,
            hub_id: HubId,
            rates: &[crate::domain::product_price_level::NewProductPriceLevelRate],
        ) -> RepositoryResult<()> {
            self.product_writer
                .create_product_price_levels(hub_id, rates)
        }

        fn replace_product_tags(
            &self,
            product_id: ProductId,
            hub_id: HubId,
            tag_ids: &[crate::domain::types::TagId],
        ) -> RepositoryResult<()> {
            self.product_writer
                .replace_product_tags(product_id, hub_id, tag_ids)
        }

        fn replace_product_images(
            &self,
            product_id: ProductId,
            hub_id: HubId,
            image_urls: &[crate::domain::types::ImageUrl],
        ) -> RepositoryResult<()> {
            self.product_writer
                .replace_product_images(product_id, hub_id, image_urls)
        }
    }

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_level(id: i32, hub_id: i32, name: &str) -> PriceLevel {
        PriceLevel {
            id: PriceLevelId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: PriceLevelName::new(name).unwrap(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            is_default: false,
        }
    }

    fn sample_customer(id: i32, hub_id: i32, price_level_id: Option<i32>) -> Customer {
        Customer {
            id: CustomerId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: CustomerName::new(format!("Customer {id}")).unwrap(),
            phone: PhoneNumber::new(format!("+100000{id}")).unwrap(),
            price_level_id: price_level_id.map(|value| PriceLevelId::new(value).unwrap()),
            public_id: None,
        }
    }

    fn sample_category(id: i32, hub_id: i32, name: &str, is_archived: bool) -> Category {
        Category {
            id: CategoryId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            parent_id: None,
            name: CategoryName::new(name).unwrap(),
            description: None,
            is_archived,
            image_url: None,
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

    fn sample_product(
        id: i32,
        hub_id: i32,
        category_id: Option<i32>,
        price_levels: Vec<(i32, i32)>,
    ) -> Product {
        let product_id = ProductId::new(id).unwrap();
        let price_levels = price_levels
            .into_iter()
            .enumerate()
            .map(|(idx, (level_id, price_cents))| ProductPriceLevelRate {
                id: ProductPriceLevelRateId::new((idx + 1) as i32).unwrap(),
                product_id,
                price_level_id: PriceLevelId::new(level_id).unwrap(),
                price_cents: PriceCents::new(price_cents).unwrap(),
                created_at: fixed_datetime(),
                updated_at: fixed_datetime(),
            })
            .collect();

        Product {
            id: product_id,
            hub_id: HubId::new(hub_id).unwrap(),
            name: ProductName::new(format!("Product {id}")).unwrap(),
            sku: None,
            description: None,
            units: None,
            currency: CurrencyCode::new("USD").unwrap(),
            is_archived: false,
            category_id: category_id.map(|value| CategoryId::new(value).unwrap()),
            vendor_id: None,
            price_levels,
            tags: Vec::new(),
            image_urls: Vec::new(),
            amount: None,
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
        let repo =
            CombinedPriceLevelRepo::new(MockPriceLevelReader::new(), MockCategoryReader::new());
        let user = user_with_roles(&[]);

        let result = load_price_levels(PriceLevelsQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_price_level_for_edit_requires_role() {
        let repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[]);

        let result = load_price_level_for_edit(3, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_price_level_for_edit_returns_not_found() {
        let mut repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_get_price_level_by_id()
            .times(1)
            .withf(|price_level_id, hub_id| {
                assert_eq!(price_level_id.get(), 9);
                assert_eq!(hub_id.get(), 42);
                true
            })
            .returning(|_, _| Ok(None));

        let result = load_price_level_for_edit(9, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_price_level_for_edit_returns_price_level() {
        let mut repo = MockPriceLevelReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_get_price_level_by_id()
            .times(1)
            .withf(|price_level_id, hub_id| {
                assert_eq!(price_level_id.get(), 11);
                assert_eq!(hub_id.get(), 42);
                true
            })
            .returning(|_, _| Ok(Some(sample_level(11, 42, "Retail"))));

        let result = load_price_level_for_edit(11, &user, &repo).expect("expected price level");

        assert_eq!(result.id.get(), 11);
        assert_eq!(result.name.as_str(), "Retail");
    }

    #[test]
    fn load_price_levels_returns_paginated_data() {
        let mut price_reader = MockPriceLevelReader::new();
        let mut category_reader = MockCategoryReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = PriceLevelsQuery {
            search: Some("sil".to_string()),
        };

        let expected_hub = user.hub_id;

        price_reader
            .expect_list_price_levels()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id.get(), expected_hub);
                assert_eq!(query.search.as_deref(), Some("sil"));
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

        category_reader
            .expect_list_categories()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.hub_id.get(), expected_hub);
                true
            })
            .returning(move |_| {
                Ok((
                    2,
                    vec![
                        sample_category(3, expected_hub, "Desserts", false),
                        sample_category(4, expected_hub, "Archived", true),
                    ],
                ))
            });

        let repo = CombinedPriceLevelRepo::new(price_reader, category_reader);
        let result = load_price_levels(query, &user, &repo);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.search.as_deref(), Some("sil"));
        assert_eq!(data.price_levels.len(), 2);
        assert_eq!(data.categories.len(), 1);
    }

    #[test]
    fn create_price_level_requires_role() {
        let repo = CombinedPriceLevelCreateRepo::new(
            MockPriceLevelWriter::new(),
            MockProductReader::new(),
            MockProductWriter::new(),
        );
        let user = user_with_roles(&[]);
        let form = AddPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let result = create_price_level(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn create_price_level_persists_price_level() {
        let mut price_writer = MockPriceLevelWriter::new();
        let mut product_reader = MockProductReader::new();
        let mut product_writer = MockProductWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let expected_hub = user.hub_id;
        price_writer
            .expect_create_price_level()
            .times(1)
            .withf(move |payload| {
                payload.hub_id.get() == expected_hub && payload.name.as_str() == "Retail"
            })
            .returning(move |_| Ok(sample_level(5, expected_hub, "Retail")));

        product_reader
            .expect_list_products()
            .times(1)
            .withf(move |query| query.hub_id.get() == expected_hub)
            .returning(|_| Ok((0, Vec::new())));

        product_writer.expect_create_product_price_levels().times(0);

        let repo = CombinedPriceLevelCreateRepo::new(price_writer, product_reader, product_writer);
        let result = create_price_level(form, &user, &repo).expect("expected success");

        assert_eq!(result.id.get(), 5);
        assert_eq!(result.hub_id.get(), expected_hub);
        assert_eq!(result.name.as_str(), "Retail");
    }

    #[test]
    fn create_price_level_propagates_form_errors() {
        let repo = CombinedPriceLevelCreateRepo::new(
            MockPriceLevelWriter::new(),
            MockProductReader::new(),
            MockProductWriter::new(),
        );
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "   ".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let result = create_price_level(form, &user, &repo);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(
                    message.contains("cannot be empty"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn create_price_level_rejects_invalid_modifier_before_persisting() {
        let mut price_writer = MockPriceLevelWriter::new();
        let mut product_reader = MockProductReader::new();
        let mut product_writer = MockProductWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "Bad Discount".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: -100,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: Vec::new(),
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let expected_hub = user.hub_id;
        price_writer.expect_create_price_level().times(0);

        product_reader
            .expect_list_products()
            .times(1)
            .withf(move |query| query.hub_id.get() == expected_hub)
            .returning(|_| Ok((1, vec![sample_product(1, 42, None, vec![(1, 1000)])])));

        product_writer.expect_create_product_price_levels().times(0);

        let repo = CombinedPriceLevelCreateRepo::new(price_writer, product_reader, product_writer);
        let result = create_price_level(form, &user, &repo);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(
                    message.contains("non-positive price"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn create_price_level_applies_modifier_for_included_categories() {
        let mut price_writer = MockPriceLevelWriter::new();
        let mut product_reader = MockProductReader::new();
        let mut product_writer = MockProductWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "Bulk".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 10,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: vec![11],
            excluded_product_ids: Vec::new(),
            included_product_ids: Vec::new(),
        };

        let expected_hub = user.hub_id;
        price_writer
            .expect_create_price_level()
            .times(1)
            .returning(move |_| Ok(sample_level(99, expected_hub, "Bulk")));

        product_reader
            .expect_list_products()
            .times(1)
            .withf(move |query| query.hub_id.get() == expected_hub)
            .returning(|_| {
                Ok((
                    4,
                    vec![
                        sample_product(1, 42, Some(10), vec![(1, 1000)]),
                        sample_product(2, 42, Some(11), vec![(1, 2000)]),
                        sample_product(3, 42, None, vec![(1, 1500)]),
                        sample_product(4, 42, Some(12), vec![(2, 1200)]),
                    ],
                ))
            });

        product_writer
            .expect_create_product_price_levels()
            .times(1)
            .withf(|hub_id, rates| {
                assert_eq!(hub_id.get(), 42);
                assert_eq!(rates.len(), 2);
                let mut totals = rates
                    .iter()
                    .map(|rate| (rate.product_id.get(), rate.price_cents.get()))
                    .collect::<Vec<_>>();
                totals.sort();
                assert_eq!(totals, vec![(1, 1100), (3, 1650)]);
                rates.iter().all(|rate| rate.price_level_id.get() == 99)
            })
            .returning(|_, _| Ok(()));

        let repo = CombinedPriceLevelCreateRepo::new(price_writer, product_reader, product_writer);
        let result = create_price_level(form, &user, &repo).expect("expected success");

        assert_eq!(result.id.get(), 99);
    }

    #[test]
    fn create_price_level_excludes_products_by_id() {
        let mut price_writer = MockPriceLevelWriter::new();
        let mut product_reader = MockProductReader::new();
        let mut product_writer = MockProductWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "VIP".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 20,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: Vec::new(),
            excluded_product_ids: vec![2, 4],
            included_product_ids: Vec::new(),
        };

        let expected_hub = user.hub_id;
        price_writer
            .expect_create_price_level()
            .times(1)
            .returning(move |_| Ok(sample_level(77, expected_hub, "VIP")));

        product_reader
            .expect_list_products()
            .times(1)
            .withf(move |query| query.hub_id.get() == expected_hub)
            .returning(|_| {
                Ok((
                    4,
                    vec![
                        sample_product(1, 42, Some(10), vec![(1, 1000)]),
                        sample_product(2, 42, Some(10), vec![(1, 2000)]),
                        sample_product(3, 42, None, vec![(1, 1500)]),
                        sample_product(4, 42, Some(12), vec![(1, 1200)]),
                    ],
                ))
            });

        product_writer
            .expect_create_product_price_levels()
            .times(1)
            .withf(|hub_id, rates| {
                assert_eq!(hub_id.get(), 42);
                assert_eq!(rates.len(), 2);
                let mut totals = rates
                    .iter()
                    .map(|rate| (rate.product_id.get(), rate.price_cents.get()))
                    .collect::<Vec<_>>();
                totals.sort();
                assert_eq!(totals, vec![(1, 1200), (3, 1800)]);
                rates.iter().all(|rate| rate.price_level_id.get() == 77)
            })
            .returning(|_, _| Ok(()));

        let repo = CombinedPriceLevelCreateRepo::new(price_writer, product_reader, product_writer);
        let result = create_price_level(form, &user, &repo).expect("expected success");

        assert_eq!(result.id.get(), 77);
    }

    #[test]
    fn create_price_level_includes_products_over_exclusions() {
        let mut price_writer = MockPriceLevelWriter::new();
        let mut product_reader = MockProductReader::new();
        let mut product_writer = MockProductWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddPriceLevelForm {
            name: "Special".to_string(),
            default: false,
            base_price_level_id: 1,
            price_modifier: 15,
            price_modifier_kind: PriceModifierKind::Percent,
            excluded_category_ids: vec![10],
            excluded_product_ids: vec![2],
            included_product_ids: vec![2],
        };

        let expected_hub = user.hub_id;
        price_writer
            .expect_create_price_level()
            .times(1)
            .returning(move |_| Ok(sample_level(88, expected_hub, "Special")));

        product_reader
            .expect_list_products()
            .times(1)
            .withf(move |query| query.hub_id.get() == expected_hub)
            .returning(|_| {
                Ok((
                    3,
                    vec![
                        sample_product(1, 42, Some(10), vec![(1, 1000)]),
                        sample_product(2, 42, Some(10), vec![(1, 2000)]),
                        sample_product(3, 42, Some(11), vec![(1, 1500)]),
                    ],
                ))
            });

        product_writer
            .expect_create_product_price_levels()
            .times(1)
            .withf(|hub_id, rates| {
                assert_eq!(hub_id.get(), 42);
                assert_eq!(rates.len(), 2);
                let mut totals = rates
                    .iter()
                    .map(|rate| (rate.product_id.get(), rate.price_cents.get()))
                    .collect::<Vec<_>>();
                totals.sort();
                assert_eq!(totals, vec![(2, 2300), (3, 1725)]);
                rates.iter().all(|rate| rate.price_level_id.get() == 88)
            })
            .returning(|_, _| Ok(()));

        let repo = CombinedPriceLevelCreateRepo::new(price_writer, product_reader, product_writer);
        let result = create_price_level(form, &user, &repo).expect("expected success");

        assert_eq!(result.id.get(), 88);
    }

    #[test]
    fn update_price_level_requires_role() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[]);
        let form = EditPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
        };

        let result = update_price_level(7, form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_price_level_updates_record() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditPriceLevelForm {
            name: "  Retail Plus  ".to_string(),
            default: true,
        };

        let expected_hub = user.hub_id;
        repo.expect_update_price_level()
            .times(1)
            .withf(move |id, hub, updates| {
                id.get() == 7
                    && hub.get() == expected_hub
                    && updates.name.as_str() == "Retail Plus"
                    && updates.is_default
            })
            .return_once(move |_, _, _| Ok(sample_level(7, expected_hub, "Retail Plus")));

        let result = update_price_level(7, form, &user, &repo).expect("expected success");

        assert_eq!(result.id.get(), 7);
        assert_eq!(result.name.as_str(), "Retail Plus");
    }

    #[test]
    fn update_price_level_propagates_form_errors() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditPriceLevelForm {
            name: "   ".to_string(),
            default: false,
        };

        let result = update_price_level(3, form, &user, &repo);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(
                    message.contains("cannot be empty"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn update_price_level_bubbles_not_found() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = EditPriceLevelForm {
            name: "Retail".to_string(),
            default: false,
        };

        repo.expect_update_price_level()
            .times(1)
            .return_once(|_, _, _| Err(RepositoryError::NotFound));

        let result = update_price_level(11, form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    struct ClientAssignmentRepo {
        customer_reader: MockCustomerReader,
        price_level_reader: MockPriceLevelReader,
    }

    impl ClientAssignmentRepo {
        fn new() -> Self {
            Self {
                customer_reader: MockCustomerReader::new(),
                price_level_reader: MockPriceLevelReader::new(),
            }
        }
    }

    impl CustomerReader for ClientAssignmentRepo {
        fn get_customer_by_id(
            &self,
            id: CustomerId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_id(id, hub_id)
        }

        fn get_customer_by_phone(
            &self,
            phone: &PhoneNumber,
            hub_id: HubId,
        ) -> RepositoryResult<Option<Customer>> {
            self.customer_reader.get_customer_by_phone(phone, hub_id)
        }

        fn list_customers(
            &self,
            query: CustomerListQuery,
        ) -> RepositoryResult<(usize, Vec<Customer>)> {
            self.customer_reader.list_customers(query)
        }
    }

    impl PriceLevelReader for ClientAssignmentRepo {
        fn get_price_level_by_id(
            &self,
            id: PriceLevelId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<PriceLevel>> {
            self.price_level_reader.get_price_level_by_id(id, hub_id)
        }

        fn list_price_levels(
            &self,
            query: PriceLevelListQuery,
        ) -> RepositoryResult<(usize, Vec<PriceLevel>)> {
            self.price_level_reader.list_price_levels(query)
        }
    }

    #[test]
    fn load_client_price_level_assignments_requires_role() {
        let repo = ClientAssignmentRepo::new();
        let user = user_with_roles(&[]);

        let result = load_client_price_level_assignments(&user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_client_price_level_assignments_returns_assignments() {
        let mut repo = ClientAssignmentRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;

        repo.price_level_reader
            .expect_list_price_levels()
            .withf(move |query| query.hub_id.get() == hub_id)
            .returning(move |_| {
                Ok((
                    2,
                    vec![
                        PriceLevel {
                            is_default: true,
                            ..sample_level(10, hub_id, "Retail")
                        },
                        sample_level(11, hub_id, "Wholesale"),
                    ],
                ))
            });

        repo.customer_reader
            .expect_list_customers()
            .withf(move |query| query.hub_id.get() == hub_id)
            .returning(move |_| {
                Ok((
                    2,
                    vec![
                        sample_customer(1, hub_id, Some(11)),
                        sample_customer(2, hub_id, None),
                    ],
                ))
            });

        let assignments =
            load_client_price_level_assignments(&user, &repo).expect("expected success");

        assert_eq!(assignments.hub_id, hub_id);
        assert_eq!(assignments.default_price_level_id, Some(10));
        assert_eq!(assignments.assignments.len(), 2);
        assert_eq!(
            assignments.assignments[0],
            ClientPriceLevelAssignment {
                phone: "+1000001".to_string(),
                price_level_id: Some(11),
            }
        );
        assert_eq!(
            assignments.assignments[1],
            ClientPriceLevelAssignment {
                phone: "+1000002".to_string(),
                price_level_id: None,
            }
        );
    }

    #[test]
    fn assign_price_level_to_client_requires_role() {
        let repo = CombinedCustomerRepo::new(MockCustomerReader::new(), MockCustomerWriter::new());
        let user = user_with_roles(&[]);
        let payload = AssignClientPriceLevelForm {
            name: "Client Example".to_string(),
            phone: "+1234567890".to_string(),
            price_level_id: Some(5),
            public_id: "public-123".to_string(),
        };

        let result = assign_price_level_to_client(payload, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn assign_price_level_to_client_updates_assignment_using_contact_lookup() {
        let mut reader = MockCustomerReader::new();
        let mut writer = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;
        let expected_customer_id = 321;
        let expected_public_id = "public-456";

        reader
            .expect_get_customer_by_phone()
            .times(1)
            .withf(move |phone, query_hub_id| {
                query_hub_id.get() == hub_id && phone.as_str() == "+15550007"
            })
            .returning(move |_, _| Ok(Some(sample_customer(expected_customer_id, hub_id, None))));

        writer
            .expect_update_customer()
            .times(1)
            .withf(move |customer_id, target_hub, updates| {
                customer_id.get() == expected_customer_id
                    && target_hub.get() == hub_id
                    && updates.name.as_str() == "Customer Seven"
                    && updates.price_level_id.map(|id| id.get()) == Some(8)
                    && updates
                        .public_id
                        .as_ref()
                        .is_some_and(|id| id.as_str() == expected_public_id)
            })
            .returning(move |_, _, _| Ok(sample_customer(expected_customer_id, hub_id, Some(8))));

        let repo = CombinedCustomerRepo::new(reader, writer);
        let payload = AssignClientPriceLevelForm {
            name: "Customer Seven".to_string(),
            phone: "  +15550007 ".to_string(),
            price_level_id: Some(8),
            public_id: expected_public_id.to_string(),
        };

        assign_price_level_to_client(payload, &user, &repo).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_clears_assignment() {
        let mut reader = MockCustomerReader::new();
        let mut writer = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;
        let expected_public_id = "public-789";
        let expected_customer = Customer {
            id: CustomerId::new(55).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: CustomerName::new("Client 55").unwrap(),
            phone: PhoneNumber::new("+1999555").unwrap(),
            price_level_id: Some(PriceLevelId::new(12).unwrap()),
            public_id: None,
        };
        let expected_customer_id = expected_customer.id;
        let expected_customer_reader = expected_customer.clone();
        let expected_customer_writer = expected_customer.clone();

        reader
            .expect_get_customer_by_phone()
            .times(1)
            .withf(move |phone, query_hub_id| {
                query_hub_id.get() == hub_id && phone.as_str() == "+1999555"
            })
            .returning(move |_, _| Ok(Some(expected_customer_reader.clone())));

        writer
            .expect_update_customer()
            .times(1)
            .withf(move |customer_id, target_hub, updates| {
                *customer_id == expected_customer_id
                    && target_hub.get() == hub_id
                    && updates.name.as_str() == "Client 55"
                    && updates.price_level_id.is_none()
                    && updates
                        .public_id
                        .as_ref()
                        .is_some_and(|id| id.as_str() == expected_public_id)
            })
            .returning(move |_, _, _| Ok(expected_customer_writer.clone()));

        let repo = CombinedCustomerRepo::new(reader, writer);
        let payload = AssignClientPriceLevelForm {
            name: "Client 55".to_string(),
            phone: "+1999555".to_string(),
            price_level_id: None,
            public_id: expected_public_id.to_string(),
        };

        assign_price_level_to_client(payload, &user, &repo).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_creates_customer_when_lookup_missing() {
        let mut reader = MockCustomerReader::new();
        let mut writer = MockCustomerWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;
        let expected_customer_id = 777;
        let expected_public_id = "public-000";

        reader
            .expect_get_customer_by_phone()
            .times(1)
            .returning(|_, _| Ok(None));

        writer
            .expect_create_customer()
            .times(1)
            .withf(move |new_customer| {
                new_customer.hub_id.get() == hub_id
                    && new_customer.name.as_str() == "Missing User"
                    && new_customer.phone.as_str() == "+1999000"
                    && new_customer.price_level_id == Some(PriceLevelId::new(1).unwrap())
                    && new_customer
                        .public_id
                        .as_ref()
                        .is_some_and(|id| id.as_str() == expected_public_id)
            })
            .returning(move |new_customer| {
                Ok(Customer {
                    id: CustomerId::new(expected_customer_id).unwrap(),
                    hub_id: HubId::new(hub_id).unwrap(),
                    name: new_customer.name.clone(),
                    phone: new_customer.phone.clone(),
                    price_level_id: new_customer.price_level_id,
                    public_id: new_customer.public_id.clone(),
                })
            });

        let repo = CombinedCustomerRepo::new(reader, writer);
        let payload = AssignClientPriceLevelForm {
            name: "  Missing User  ".to_string(),
            phone: " +1999000 ".to_string(),
            price_level_id: Some(1),
            public_id: expected_public_id.to_string(),
        };

        assign_price_level_to_client(payload, &user, &repo).expect("expected success");
    }

    #[test]
    fn assign_price_level_to_client_propagates_form_errors() {
        let repo = CombinedCustomerRepo::new(MockCustomerReader::new(), MockCustomerWriter::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let payload = AssignClientPriceLevelForm {
            name: "".to_string(),
            phone: "".to_string(),
            price_level_id: Some(0),
            public_id: "".to_string(),
        };

        let result = assign_price_level_to_client(payload, &user, &repo);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(message.contains("validation failed:"));
                assert!(message.contains("price_level_id: Validation error: range"));
                assert!(message.contains("phone: Validation error: length"));
                assert!(message.contains("name: Validation error: length"));
                assert!(message.contains("public_id: Validation error: length"));
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn remove_price_level_requires_role() {
        let repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[]);

        let result = remove_price_level(42, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn remove_price_level_bubbles_not_found() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_price_level()
            .times(1)
            .withf(|id, hub| id.get() == 99 && hub.get() == 42)
            .return_once(|_, _| Err(RepositoryError::NotFound));

        let result = remove_price_level(99, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn remove_price_level_succeeds() {
        let mut repo = MockPriceLevelWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        repo.expect_delete_price_level()
            .times(1)
            .withf(|id, hub| id.get() == 7 && hub.get() == 42)
            .return_once(|_, _| Ok(()));

        remove_price_level(7, &user, &repo).expect("expected success");
    }
}
