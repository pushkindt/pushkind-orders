//! Product repository implementation with Diesel and FTS support.

use std::collections::{HashMap, HashSet};

use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text};
use diesel::sqlite::SqliteConnection;
use pushkind_common::repository::build_fts_match_query;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::product::{
        NewProduct as DomainNewProduct, Product as DomainProduct, ProductListQuery,
        UpdateProduct as DomainUpdateProduct,
    },
    domain::product_price_level::{
        NewProductPriceLevelRate as DomainNewProductPriceLevelRate,
        ProductPriceLevelRate as DomainProductPriceLevelRate,
    },
    domain::product_tag::NewProductTag as DomainNewProductTag,
    domain::tag::Tag as DomainTag,
    domain::types::{HubId, ImageUrl, ProductId, TagId},
    models::product::{
        NewProduct as DbNewProduct, Product as DbProduct, UpdateProduct as DbUpdateProduct,
    },
    models::product_image::{NewProductImage as DbNewProductImage, ProductImage as DbProductImage},
    models::product_price_level::{
        NewProductPriceLevel as DbNewProductPriceLevel, ProductPriceLevel as DbProductPriceLevel,
    },
    models::product_tag::{NewProductTag as DbNewProductTag, ProductTag as DbProductTag},
    models::tag::Tag as DbTag,
    repository::{DieselRepository, ProductReader, ProductWriter},
};

impl ProductReader for DieselRepository {
    fn get_product_by_id(
        &self,
        id: ProductId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainProduct>> {
        use crate::schema::products;

        let mut conn = self.conn()?;
        let product = products::table
            .filter(products::id.eq(id.get()))
            .filter(products::hub_id.eq(hub_id.get()))
            .first::<DbProduct>(&mut conn)
            .optional()?;

        let product = product.map(DomainProduct::try_from).transpose()?;

        if let Some(mut domain) = product {
            let mut price_levels = load_price_levels_for_products(&mut conn, &[domain.id])?;
            domain.price_levels = price_levels.remove(&domain.id).unwrap_or_default();
            let mut tags = load_tags_for_products(&mut conn, &[domain.id])?;
            domain.tags = tags.remove(&domain.id).unwrap_or_default();
            let mut images = load_image_urls_for_products(&mut conn, &[domain.id])?;
            domain.image_urls = images.remove(&domain.id).unwrap_or_default();
            Ok(Some(domain))
        } else {
            Ok(None)
        }
    }

    fn list_products(
        &self,
        query: ProductListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainProduct>)> {
        use crate::schema::{product_fts, product_tags, products, tags};

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = products::table
                .filter(products::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();
            if !query.include_archived {
                items = items.filter(products::is_archived.eq(false));
            }

            if query.only_without_category {
                items = items.filter(products::category_id.is_null());
            }

            if let Some(category_id) = query.category_id {
                items = items.filter(products::category_id.eq(Some(category_id.get())));
            }

            if let Some(tag_id) = query.tag_id {
                let tagged_product_ids = product_tags::table
                    .inner_join(tags::table)
                    .select(product_tags::product_id)
                    .filter(tags::id.eq(tag_id.get()))
                    .filter(tags::hub_id.eq(query.hub_id.get()));

                items = items.filter(products::id.eq_any(tagged_product_ids));
            }

            if let Some(term) = query.search.as_ref()
                && let Some(fts_query) = build_fts_match_query(term)
            {
                let fts_filter = exists(
                    product_fts::table
                        .filter(product_fts::rowid.eq(products::id))
                        .filter(
                            diesel::dsl::sql::<Bool>("product_fts MATCH ")
                                .bind::<Text, _>(fts_query),
                        ),
                );
                items = items.filter(fts_filter);
            }

            if let Some(sku) = query.sku.as_ref() {
                items = items.filter(products::sku.eq(sku.as_str()));
            }
            items
        };

        // Get the total count before applying pagination
        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items =
            query_builder().order((products::is_archived.asc(), products::created_at.desc()));

        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_products = items.load::<DbProduct>(&mut conn)?;

        if db_products.is_empty() {
            return Ok((total, Vec::new()));
        }

        let mut domain_products: Vec<DomainProduct> = db_products
            .into_iter()
            .map(DomainProduct::try_from)
            .collect::<Result<_, _>>()?;

        let product_ids: Vec<_> = domain_products.iter().map(|product| product.id).collect();
        let mut price_level_map = load_price_levels_for_products(&mut conn, &product_ids)?;
        let mut tag_map = load_tags_for_products(&mut conn, &product_ids)?;
        let mut image_map = load_image_urls_for_products(&mut conn, &product_ids)?;

        for product in &mut domain_products {
            product.price_levels = price_level_map.remove(&product.id).unwrap_or_default();
            product.tags = tag_map.remove(&product.id).unwrap_or_default();
            product.image_urls = image_map.remove(&product.id).unwrap_or_default();
        }

        Ok((total, domain_products))
    }
}

impl ProductWriter for DieselRepository {
    fn create_product(&self, new_product: &DomainNewProduct) -> RepositoryResult<DomainProduct> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        if let Some(category_id) = new_product.category_id {
            use crate::schema::categories;
            use diesel::dsl::{exists, select};

            let category_exists: bool = select(exists(
                categories::table
                    .filter(categories::id.eq(category_id.get()))
                    .filter(categories::hub_id.eq(new_product.hub_id.get())),
            ))
            .get_result(&mut conn)?;

            if !category_exists {
                return Err(RepositoryError::NotFound);
            }
        }

        let db_new = DbNewProduct::from(new_product);

        let created = diesel::insert_into(products::table)
            .values(&db_new)
            .get_result::<DbProduct>(&mut conn)?;

        let mut domain: DomainProduct = created.try_into()?;
        let mut price_levels = load_price_levels_for_products(&mut conn, &[domain.id])?;
        domain.price_levels = price_levels.remove(&domain.id).unwrap_or_default();
        let mut tags = load_tags_for_products(&mut conn, &[domain.id])?;
        domain.tags = tags.remove(&domain.id).unwrap_or_default();
        let mut images = load_image_urls_for_products(&mut conn, &[domain.id])?;
        domain.image_urls = images.remove(&domain.id).unwrap_or_default();

        Ok(domain)
    }

    fn update_product(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        updates: &DomainUpdateProduct,
    ) -> RepositoryResult<DomainProduct> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        if let Some(category_id) = updates.category_id {
            use crate::schema::categories;
            use diesel::dsl::{exists, select};

            let category_exists: bool = select(exists(
                categories::table
                    .filter(categories::id.eq(category_id.get()))
                    .filter(categories::hub_id.eq(hub_id.get())),
            ))
            .get_result(&mut conn)?;

            if !category_exists {
                return Err(RepositoryError::NotFound);
            }
        }

        let db_updates = DbUpdateProduct::from(updates);

        let target = products::table
            .filter(products::id.eq(product_id.get()))
            .filter(products::hub_id.eq(hub_id.get()));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbProduct>(&mut conn)?;

        let mut domain: DomainProduct = updated.try_into()?;
        let mut price_levels = load_price_levels_for_products(&mut conn, &[domain.id])?;
        domain.price_levels = price_levels.remove(&domain.id).unwrap_or_default();
        let mut tags = load_tags_for_products(&mut conn, &[domain.id])?;
        domain.tags = tags.remove(&domain.id).unwrap_or_default();
        let mut images = load_image_urls_for_products(&mut conn, &[domain.id])?;
        domain.image_urls = images.remove(&domain.id).unwrap_or_default();

        Ok(domain)
    }

    fn delete_product(&self, product_id: ProductId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        let target = products::table
            .filter(products::id.eq(product_id.get()))
            .filter(products::hub_id.eq(hub_id.get()));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    fn replace_product_price_levels(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        rates: &[DomainNewProductPriceLevelRate],
    ) -> RepositoryResult<()> {
        use crate::schema::price_levels;
        use crate::schema::product_price_levels;
        use crate::schema::products;
        use diesel::dsl::{delete, exists};
        use diesel::dsl::{insert_into, select};

        let mut conn = self.conn()?;

        conn.transaction::<_, RepositoryError, _>(|conn| {
            let is_owned: bool = select(exists(
                products::table
                    .filter(products::id.eq(product_id.get()))
                    .filter(products::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;

            if !is_owned {
                return Err(RepositoryError::NotFound);
            }

            delete(
                product_price_levels::table
                    .filter(product_price_levels::product_id.eq(product_id.get())),
            )
            .execute(conn)?;

            if !rates.is_empty() {
                let price_level_ids: std::collections::BTreeSet<i32> =
                    rates.iter().map(|rate| rate.price_level_id.get()).collect();
                let expected_count = price_level_ids.len() as i64;

                if expected_count > 0 {
                    let actual_count: i64 = price_levels::table
                        .filter(price_levels::id.eq_any(price_level_ids))
                        .filter(price_levels::hub_id.eq(hub_id.get()))
                        .count()
                        .get_result(conn)?;

                    if actual_count != expected_count {
                        return Err(RepositoryError::NotFound);
                    }
                }

                let rows: Vec<DbNewProductPriceLevel> =
                    rates.iter().map(DbNewProductPriceLevel::from).collect();
                insert_into(product_price_levels::table)
                    .values(&rows)
                    .execute(conn)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    fn replace_product_tags(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        tag_ids: &[TagId],
    ) -> RepositoryResult<()> {
        use crate::schema::product_tags;
        use crate::schema::products;
        use crate::schema::tags;
        use diesel::dsl::{delete, exists, insert_into, select};

        let mut conn = self.conn()?;

        conn.transaction::<_, RepositoryError, _>(|conn| {
            let is_owned: bool = select(exists(
                products::table
                    .filter(products::id.eq(product_id.get()))
                    .filter(products::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;

            if !is_owned {
                return Err(RepositoryError::NotFound);
            }

            delete(product_tags::table.filter(product_tags::product_id.eq(product_id.get())))
                .execute(conn)?;

            if !tag_ids.is_empty() {
                let unique_ids: HashSet<TagId> = tag_ids.iter().copied().collect();

                if !unique_ids.is_empty() {
                    let raw_ids: Vec<i32> = unique_ids.iter().map(|id| id.get()).collect();
                    let expected_count = raw_ids.len() as i64;
                    let actual_count: i64 = tags::table
                        .filter(tags::id.eq_any(&raw_ids))
                        .filter(tags::hub_id.eq(hub_id.get()))
                        .count()
                        .get_result(conn)?;

                    if actual_count != expected_count {
                        return Err(RepositoryError::NotFound);
                    }

                    let rows: Vec<DbNewProductTag> = unique_ids
                        .into_iter()
                        .map(|tag_id| {
                            let domain = DomainNewProductTag::new(product_id, tag_id);
                            DbNewProductTag::from(&domain)
                        })
                        .collect();

                    if !rows.is_empty() {
                        insert_into(product_tags::table)
                            .values(&rows)
                            .execute(conn)?;
                    }
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn replace_product_images(
        &self,
        product_id: ProductId,
        hub_id: HubId,
        image_urls: &[ImageUrl],
    ) -> RepositoryResult<()> {
        use crate::schema::product_images;
        use crate::schema::products;
        use diesel::dsl::{delete, exists, insert_into, select};

        let mut conn = self.conn()?;

        conn.transaction::<_, RepositoryError, _>(|conn| {
            let is_owned: bool = select(exists(
                products::table
                    .filter(products::id.eq(product_id.get()))
                    .filter(products::hub_id.eq(hub_id.get())),
            ))
            .get_result(conn)?;

            if !is_owned {
                return Err(RepositoryError::NotFound);
            }

            delete(product_images::table.filter(product_images::product_id.eq(product_id.get())))
                .execute(conn)?;

            if !image_urls.is_empty() {
                let product_id_raw = product_id.get();
                let rows: Vec<DbNewProductImage<'_>> = image_urls
                    .iter()
                    .map(|url| DbNewProductImage {
                        product_id: product_id_raw,
                        image_url: url.as_str(),
                    })
                    .collect();

                insert_into(product_images::table)
                    .values(&rows)
                    .execute(conn)?;
            }

            Ok(())
        })?;

        Ok(())
    }
}

/// Load price level associations for multiple products.
fn load_price_levels_for_products(
    conn: &mut SqliteConnection,
    product_ids: &[ProductId],
) -> RepositoryResult<HashMap<ProductId, Vec<DomainProductPriceLevelRate>>> {
    use crate::schema::product_price_levels;

    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let raw_ids: Vec<i32> = product_ids.iter().map(|id| id.get()).collect();

    let rows = product_price_levels::table
        .filter(product_price_levels::product_id.eq_any(&raw_ids))
        .order(product_price_levels::created_at.asc())
        .load::<DbProductPriceLevel>(conn)?;

    let mut map: HashMap<ProductId, Vec<DomainProductPriceLevelRate>> = HashMap::new();
    for row in rows {
        let product_id = ProductId::new(row.product_id)?;
        map.entry(product_id)
            .or_default()
            .push(DomainProductPriceLevelRate::try_from(row)?);
    }

    Ok(map)
}

/// Load tag associations for multiple products.
fn load_tags_for_products(
    conn: &mut SqliteConnection,
    product_ids: &[ProductId],
) -> RepositoryResult<HashMap<ProductId, Vec<DomainTag>>> {
    use crate::schema::product_tags;
    use crate::schema::tags;

    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let raw_ids: Vec<i32> = product_ids.iter().map(|id| id.get()).collect();

    let rows = product_tags::table
        .inner_join(tags::table)
        .filter(product_tags::product_id.eq_any(&raw_ids))
        .order(tags::name.asc())
        .load::<(DbProductTag, DbTag)>(conn)?;

    let mut map: HashMap<ProductId, Vec<DomainTag>> = HashMap::new();
    for (link, tag) in rows {
        let product_id = ProductId::new(link.product_id)?;
        map.entry(product_id)
            .or_default()
            .push(DomainTag::try_from(tag)?);
    }

    Ok(map)
}

/// Load image URLs for multiple products.
fn load_image_urls_for_products(
    conn: &mut SqliteConnection,
    product_ids: &[ProductId],
) -> RepositoryResult<HashMap<ProductId, Vec<ImageUrl>>> {
    use crate::schema::product_images;

    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let raw_ids: Vec<i32> = product_ids.iter().map(|id| id.get()).collect();

    let rows = product_images::table
        .filter(product_images::product_id.eq_any(&raw_ids))
        .order(product_images::id.asc())
        .load::<DbProductImage>(conn)?;

    let mut map: HashMap<ProductId, Vec<ImageUrl>> = HashMap::new();
    for row in rows {
        let product_id = ProductId::new(row.product_id)?;
        map.entry(product_id)
            .or_default()
            .push(ImageUrl::new(row.image_url)?);
    }

    Ok(map)
}
