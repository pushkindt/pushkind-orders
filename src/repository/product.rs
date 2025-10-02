use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::product::{
        NewProduct as DomainNewProduct, Product as DomainProduct, ProductListQuery,
        UpdateProduct as DomainUpdateProduct,
    },
    models::product::{
        NewProduct as DbNewProduct, Product as DbProduct, UpdateProduct as DbUpdateProduct,
    },
    repository::{DieselRepository, ProductReader, ProductWriter},
};

impl ProductReader for DieselRepository {
    fn get_product_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainProduct>> {
        use crate::schema::products;

        let mut conn = self.conn()?;
        let product = products::table
            .filter(products::id.eq(Some(id)))
            .filter(products::hub_id.eq(hub_id))
            .first::<DbProduct>(&mut conn)
            .optional()?;

        Ok(product.map(Into::into))
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

        Ok((total, db_products.into_iter().map(Into::into).collect()))
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

        Ok(created.into())
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
            .filter(products::id.eq(Some(product_id)))
            .filter(products::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbProduct>(&mut conn)?;

        Ok(updated.into())
    }

    fn delete_product(&self, product_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::products;

        let mut conn = self.conn()?;

        let target = products::table
            .filter(products::id.eq(Some(product_id)))
            .filter(products::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
