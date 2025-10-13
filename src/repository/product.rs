use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::product::{
        NewProduct as DomainNewProduct, Product as DomainProduct, ProductListQuery,
        UpdateProduct as DomainUpdateProduct,
    },
    domain::product_price_level::ProductPriceLevelRate as DomainProductPriceLevelRate,
    models::product::{
        NewProduct as DbNewProduct, Product as DbProduct, UpdateProduct as DbUpdateProduct,
    },
    models::product_price_level::ProductPriceLevel as DbProductPriceLevel,
    repository::{DieselRepository, ProductReader, ProductWriter},
};

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
            Ok(Some(domain))
        } else {
            Ok(None)
        }
    }

    fn list_products(
        &self,
        query: ProductListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainProduct>)> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        let mut count_query = products::table
            .filter(products::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if !query.include_archived {
            count_query = count_query.filter(products::is_archived.eq(false));
        }

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            count_query = count_query.filter(
                products::name
                    .like(pattern.clone())
                    .or(products::description.like(pattern)),
            );
        }

        if let Some(sku) = query.sku.as_ref() {
            count_query = count_query.filter(products::sku.eq(sku));
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items = products::table
            .filter(products::hub_id.eq(query.hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if !query.include_archived {
            items = items.filter(products::is_archived.eq(false));
        }

        if let Some(term) = query.search.as_ref() {
            let pattern = format!("%{}%", term);
            items = items.filter(
                products::name
                    .like(pattern.clone())
                    .or(products::description.like(pattern)),
            );
        }

        if let Some(sku) = query.sku.as_ref() {
            items = items.filter(products::sku.eq(sku));
        }

        items = items.order((products::is_archived.asc(), products::created_at.desc()));

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

        let mut domain_products = Vec::with_capacity(db_products.len());
        for db_product in db_products {
            let mut domain: DomainProduct = db_product.into();
            domain.price_levels = price_level_map.remove(&domain.id).unwrap_or_default();
            domain_products.push(domain);
        }

        Ok((total, domain_products))
    }
}

impl ProductWriter for DieselRepository {
    fn create_product(&self, new_product: &DomainNewProduct) -> RepositoryResult<DomainProduct> {
        use crate::schema::products;

        let mut conn = self.conn()?;
        let db_new = DbNewProduct::from(new_product);

        let created = diesel::insert_into(products::table)
            .values(&db_new)
            .get_result::<DbProduct>(&mut conn)?;

        let mut domain: DomainProduct = created.into();
        let mut price_levels = load_price_levels_for_products(&mut conn, &[domain.id])?;
        domain.price_levels = price_levels.remove(&domain.id).unwrap_or_default();

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
        map.entry(row.product_id).or_default().push(row.into());
    }

    Ok(map)
}
