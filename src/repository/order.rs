use std::collections::HashMap;

use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::order::{
        NewOrder as DomainNewOrder, Order as DomainOrder, OrderListQuery,
        UpdateOrder as DomainUpdateOrder,
    },
    models::order::{
        NewOrder as DbNewOrder, NewOrderProduct as DbNewOrderProduct, Order as DbOrder,
        OrderProduct as DbOrderProduct, UpdateOrder as DbUpdateOrder,
    },
    repository::{DieselRepository, OrderReader, OrderWriter},
};

impl OrderReader for DieselRepository {
    fn get_order_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainOrder>> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;
        let order = orders::table
            .filter(orders::id.eq(id))
            .filter(orders::hub_id.eq(hub_id))
            .first::<DbOrder>(&mut conn)
            .optional()?;

        let Some(order) = order else {
            return Ok(None);
        };

        let order_id = order.id;

        let products = order_products::table
            .filter(order_products::order_id.eq(order_id))
            .order(order_products::id.asc())
            .load::<DbOrderProduct>(&mut conn)?;

        Ok(Some(DomainOrder::from((order, products))))
    }

    fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<DomainOrder>)> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;

        let OrderListQuery {
            hub_id,
            status,
            customer_id,
            search,
            pagination,
        } = query;

        let status_filter = status.map(String::from);
        let search_pattern = search.as_ref().map(|term| format!("%{}%", term));

        let mut count_query = orders::table
            .filter(orders::hub_id.eq(hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(ref status_value) = status_filter {
            count_query = count_query.filter(orders::status.eq(status_value.as_str()));
        }

        if let Some(customer) = customer_id {
            count_query = count_query.filter(orders::customer_id.eq(Some(customer)));
        }

        if let Some(ref pattern) = search_pattern {
            count_query = count_query.filter(
                orders::reference
                    .like(pattern.clone())
                    .or(orders::notes.like(pattern.clone())),
            );
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items = orders::table
            .filter(orders::hub_id.eq(hub_id))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(ref status_value) = status_filter {
            items = items.filter(orders::status.eq(status_value.as_str()));
        }

        if let Some(customer) = customer_id {
            items = items.filter(orders::customer_id.eq(Some(customer)));
        }

        if let Some(ref pattern) = search_pattern {
            items = items.filter(
                orders::reference
                    .like(pattern.clone())
                    .or(orders::notes.like(pattern.clone())),
            );
        }

        items = items.order(orders::created_at.desc());

        if let Some(pagination) = pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_orders = items.load::<DbOrder>(&mut conn)?;
        if db_orders.is_empty() {
            return Ok((total, Vec::new()));
        }

        let order_ids: Vec<i32> = db_orders.iter().map(|order| order.id).collect();

        let mut products_by_order: HashMap<i32, Vec<DbOrderProduct>> = HashMap::new();

        if !order_ids.is_empty() {
            let rows = order_products::table
                .filter(order_products::order_id.eq_any(&order_ids))
                .order(order_products::id.asc())
                .load::<DbOrderProduct>(&mut conn)?;

            for product in rows {
                products_by_order
                    .entry(product.order_id)
                    .or_default()
                    .push(product);
            }
        }

        let orders = db_orders
            .into_iter()
            .map(|order| {
                let order_id = order.id;
                let products = products_by_order.remove(&order_id).unwrap_or_default();
                DomainOrder::from((order, products))
            })
            .collect();

        Ok((total, orders))
    }
}

impl OrderWriter for DieselRepository {
    fn create_order(&self, new_order: &DomainNewOrder) -> RepositoryResult<DomainOrder> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;

        conn.transaction::<DomainOrder, RepositoryError, _>(|conn| {
            let db_new = DbNewOrder::from(new_order);

            let created = diesel::insert_into(orders::table)
                .values(&db_new)
                .get_result::<DbOrder>(conn)?;

            let order_id = created.id;

            if !new_order.products.is_empty() {
                let payload: Vec<DbNewOrderProduct> = new_order
                    .products
                    .iter()
                    .map(|product| DbNewOrderProduct::from_domain(order_id, product))
                    .collect();

                diesel::insert_into(order_products::table)
                    .values(&payload)
                    .execute(conn)?;
            }

            let products = order_products::table
                .filter(order_products::order_id.eq(order_id))
                .order(order_products::id.asc())
                .load::<DbOrderProduct>(conn)?;

            Ok(DomainOrder::from((created, products)))
        })
    }

    fn update_order(
        &self,
        order_id: i32,
        hub_id: i32,
        updates: &DomainUpdateOrder,
    ) -> RepositoryResult<DomainOrder> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;

        conn.transaction::<DomainOrder, RepositoryError, _>(|conn| {
            let db_updates = DbUpdateOrder::from(updates);

            let target = orders::table
                .filter(orders::id.eq(order_id))
                .filter(orders::hub_id.eq(hub_id));

            let updated = diesel::update(target)
                .set(&db_updates)
                .get_result::<DbOrder>(conn)?;

            if let Some(products) = updates.products.as_ref() {
                diesel::delete(order_products::table.filter(order_products::order_id.eq(order_id)))
                    .execute(conn)?;

                if !products.is_empty() {
                    let payload: Vec<DbNewOrderProduct> = products
                        .iter()
                        .map(|product| DbNewOrderProduct::from_domain(order_id, product))
                        .collect();

                    diesel::insert_into(order_products::table)
                        .values(&payload)
                        .execute(conn)?;
                }
            }

            let products = order_products::table
                .filter(order_products::order_id.eq(order_id))
                .order(order_products::id.asc())
                .load::<DbOrderProduct>(conn)?;

            Ok(DomainOrder::from((updated, products)))
        })
    }

    fn delete_order(&self, order_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::orders;

        let mut conn = self.conn()?;

        let target = orders::table
            .filter(orders::id.eq(order_id))
            .filter(orders::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
