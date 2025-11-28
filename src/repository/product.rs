use std::collections::HashMap;

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
    domain::types::TypeConstraintError,
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

fn map_tag_type_error(err: TypeConstraintError) -> RepositoryError {
    RepositoryError::Unexpected(format!("Invalid tag data: {err}"))
}

fn map_price_level_type_error(err: TypeConstraintError) -> RepositoryError {
    RepositoryError::Unexpected(format!("Invalid product price level data: {err}"))
}

impl ProductReader for DieselRepository {
    fn get_product_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainProduct>> {
        use crate::schema::products;

        let mut conn = self.conn()?;
        let product = products::table
            .filter(products::id.eq(id))
            .filter(products::hub_id.eq(hub_id))
            .first::<DbProduct>(&mut conn)
            .optional()?;

        if let Some(db_product) = product {
            let mut domain: DomainProduct = db_product.into();
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
        use crate::schema::{product_fts, products};

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = products::table
                .filter(products::hub_id.eq(query.hub_id))
                .into_boxed::<diesel::sqlite::Sqlite>();
            if !query.include_archived {
                items = items.filter(products::is_archived.eq(false));
            }

            if query.only_without_category {
                items = items.filter(products::category_id.is_null());
            }

            if let Some(category_id) = query.category_id {
                items = items.filter(products::category_id.eq(Some(category_id)));
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
                items = items.filter(products::sku.eq(sku));
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

        let product_ids: Vec<i32> = db_products.iter().map(|product| product.id).collect();
        let mut price_level_map = load_price_levels_for_products(&mut conn, &product_ids)?;
        let mut tag_map = load_tags_for_products(&mut conn, &product_ids)?;
        let mut image_map = load_image_urls_for_products(&mut conn, &product_ids)?;

        let mut domain_products = Vec::with_capacity(db_products.len());
        for db_product in db_products {
            let mut domain: DomainProduct = db_product.into();
            domain.price_levels = price_level_map.remove(&domain.id).unwrap_or_default();
            domain.tags = tag_map.remove(&domain.id).unwrap_or_default();
            domain.image_urls = image_map.remove(&domain.id).unwrap_or_default();
            domain_products.push(domain);
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
                    .filter(categories::id.eq(category_id))
                    .filter(categories::hub_id.eq(new_product.hub_id)),
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

        let mut domain: DomainProduct = created.into();
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
        product_id: i32,
        hub_id: i32,
        updates: &DomainUpdateProduct,
    ) -> RepositoryResult<DomainProduct> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        if let Some(category_id) = updates.category_id {
            use crate::schema::categories;
            use diesel::dsl::{exists, select};

            let category_exists: bool = select(exists(
                categories::table
                    .filter(categories::id.eq(category_id))
                    .filter(categories::hub_id.eq(hub_id)),
            ))
            .get_result(&mut conn)?;

            if !category_exists {
                return Err(RepositoryError::NotFound);
            }
        }

        let db_updates = DbUpdateProduct::from(updates);

        let target = products::table
            .filter(products::id.eq(product_id))
            .filter(products::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbProduct>(&mut conn)?;

        let mut domain: DomainProduct = updated.into();
        let mut price_levels = load_price_levels_for_products(&mut conn, &[domain.id])?;
        domain.price_levels = price_levels.remove(&domain.id).unwrap_or_default();
        let mut tags = load_tags_for_products(&mut conn, &[domain.id])?;
        domain.tags = tags.remove(&domain.id).unwrap_or_default();
        let mut images = load_image_urls_for_products(&mut conn, &[domain.id])?;
        domain.image_urls = images.remove(&domain.id).unwrap_or_default();

        Ok(domain)
    }

    fn delete_product(&self, product_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        let target = products::table
            .filter(products::id.eq(product_id))
            .filter(products::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    fn replace_product_price_levels(
        &self,
        product_id: i32,
        hub_id: i32,
        rates: &[DomainNewProductPriceLevelRate],
    ) -> RepositoryResult<()> {
        use crate::schema::price_levels;
        use crate::schema::product_price_levels;
        use crate::schema::products;
        use diesel::dsl::{delete, exists};
        use diesel::dsl::{insert_into, select};

        let mut conn = self.conn()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let is_owned: bool = select(exists(
                products::table
                    .filter(products::id.eq(product_id))
                    .filter(products::hub_id.eq(hub_id)),
            ))
            .get_result(conn)?;

            if !is_owned {
                return Err(diesel::result::Error::NotFound);
            }

            delete(
                product_price_levels::table.filter(product_price_levels::product_id.eq(product_id)),
            )
            .execute(conn)?;

            if !rates.is_empty() {
                let price_level_ids: std::collections::BTreeSet<i32> =
                    rates.iter().map(|rate| rate.price_level_id.get()).collect();
                let expected_count = price_level_ids.len() as i64;

                if expected_count > 0 {
                    let actual_count: i64 = price_levels::table
                        .filter(price_levels::id.eq_any(price_level_ids))
                        .filter(price_levels::hub_id.eq(hub_id))
                        .count()
                        .get_result(conn)?;

                    if actual_count != expected_count {
                        return Err(diesel::result::Error::NotFound);
                    }
                }

                let rows: Vec<DbNewProductPriceLevel> =
                    rates.iter().map(DbNewProductPriceLevel::from).collect();
                insert_into(product_price_levels::table)
                    .values(&rows)
                    .execute(conn)?;
            }

            Ok(())
        })
        .map_err(RepositoryError::from)
    }

    fn replace_product_tags(
        &self,
        product_id: i32,
        hub_id: i32,
        tag_ids: &[i32],
    ) -> RepositoryResult<()> {
        use crate::schema::product_tags;
        use crate::schema::products;
        use crate::schema::tags;
        use diesel::dsl::{delete, exists, insert_into, select};

        let mut conn = self.conn()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let is_owned: bool = select(exists(
                products::table
                    .filter(products::id.eq(product_id))
                    .filter(products::hub_id.eq(hub_id)),
            ))
            .get_result(conn)?;

            if !is_owned {
                return Err(diesel::result::Error::NotFound);
            }

            delete(product_tags::table.filter(product_tags::product_id.eq(product_id)))
                .execute(conn)?;

            if !tag_ids.is_empty() {
                let unique_ids: std::collections::BTreeSet<i32> =
                    tag_ids.iter().copied().filter(|id| *id > 0).collect();

                if !unique_ids.is_empty() {
                    let expected_count = unique_ids.len() as i64;
                    let actual_count: i64 = tags::table
                        .filter(tags::id.eq_any(&unique_ids))
                        .filter(tags::hub_id.eq(hub_id))
                        .count()
                        .get_result(conn)?;

                    if actual_count != expected_count {
                        return Err(diesel::result::Error::NotFound);
                    }

                    let rows: Vec<DbNewProductTag> = unique_ids
                        .into_iter()
                        .map(|tag_id| {
                            let domain = DomainNewProductTag::try_new(product_id, tag_id)
                                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
                            Ok::<DbNewProductTag, diesel::result::Error>(DbNewProductTag::from(
                                &domain,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    if !rows.is_empty() {
                        insert_into(product_tags::table)
                            .values(&rows)
                            .execute(conn)?;
                    }
                }
            }

            Ok(())
        })
        .map_err(RepositoryError::from)
    }

    fn replace_product_images(
        &self,
        product_id: i32,
        hub_id: i32,
        image_urls: &[String],
    ) -> RepositoryResult<()> {
        use crate::schema::product_images;
        use crate::schema::products;
        use diesel::dsl::{delete, exists, insert_into, select};

        let mut conn = self.conn()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let is_owned: bool = select(exists(
                products::table
                    .filter(products::id.eq(product_id))
                    .filter(products::hub_id.eq(hub_id)),
            ))
            .get_result(conn)?;

            if !is_owned {
                return Err(diesel::result::Error::NotFound);
            }

            delete(product_images::table.filter(product_images::product_id.eq(product_id)))
                .execute(conn)?;

            if !image_urls.is_empty() {
                let rows: Vec<DbNewProductImage<'_>> = image_urls
                    .iter()
                    .map(|url| DbNewProductImage {
                        product_id,
                        image_url: url.as_str(),
                    })
                    .collect();

                insert_into(product_images::table)
                    .values(&rows)
                    .execute(conn)?;
            }

            Ok(())
        })
        .map_err(RepositoryError::from)
    }
}

fn load_price_levels_for_products(
    conn: &mut SqliteConnection,
    product_ids: &[i32],
) -> RepositoryResult<HashMap<i32, Vec<DomainProductPriceLevelRate>>> {
    use crate::schema::product_price_levels;

    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = product_price_levels::table
        .filter(product_price_levels::product_id.eq_any(product_ids))
        .order(product_price_levels::created_at.asc())
        .load::<DbProductPriceLevel>(conn)?;

    let mut map: HashMap<i32, Vec<DomainProductPriceLevelRate>> = HashMap::new();
    for row in rows {
        map.entry(row.product_id)
            .or_default()
            .push(DomainProductPriceLevelRate::try_from(row).map_err(map_price_level_type_error)?);
    }

    Ok(map)
}

fn load_tags_for_products(
    conn: &mut SqliteConnection,
    product_ids: &[i32],
) -> RepositoryResult<HashMap<i32, Vec<DomainTag>>> {
    use crate::schema::product_tags;
    use crate::schema::tags;

    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = product_tags::table
        .inner_join(tags::table)
        .filter(product_tags::product_id.eq_any(product_ids))
        .order(tags::name.asc())
        .load::<(DbProductTag, DbTag)>(conn)?;

    let mut map: HashMap<i32, Vec<DomainTag>> = HashMap::new();
    for (link, tag) in rows {
        map.entry(link.product_id)
            .or_default()
            .push(DomainTag::try_from(tag).map_err(map_tag_type_error)?);
    }

    Ok(map)
}

fn load_image_urls_for_products(
    conn: &mut SqliteConnection,
    product_ids: &[i32],
) -> RepositoryResult<HashMap<i32, Vec<String>>> {
    use crate::schema::product_images;

    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = product_images::table
        .filter(product_images::product_id.eq_any(product_ids))
        .order(product_images::id.asc())
        .load::<DbProductImage>(conn)?;

    let mut map: HashMap<i32, Vec<String>> = HashMap::new();
    for row in rows {
        map.entry(row.product_id).or_default().push(row.image_url);
    }

    Ok(map)
}
