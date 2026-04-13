//! Order repository implementation with Diesel.

use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::dsl::exists;
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::{
        order::{
            NewOrder as DomainNewOrder, Order as DomainOrder, OrderListQuery,
            OrderProductApprovalUpdate, UpdateOrder as DomainUpdateOrder,
        },
        types::{HubId, OrderId, PriceCents, TypeConstraintError},
    },
    models::order::{
        NewOrder as DbNewOrder, NewOrderProduct as DbNewOrderProduct, Order as DbOrder,
        OrderProduct as DbOrderProduct, UpdateOrder as DbUpdateOrder,
    },
    repository::{DieselRepository, OrderReader, OrderWriter},
};

impl OrderReader for DieselRepository {
    fn get_order_by_id(&self, id: OrderId, hub_id: HubId) -> RepositoryResult<Option<DomainOrder>> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;
        let order = orders::table
            .filter(orders::id.eq(id.get()))
            .filter(orders::hub_id.eq(hub_id.get()))
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

        Ok(Some(DomainOrder::try_from((order, products))?))
    }

    fn list_orders(&self, query: OrderListQuery) -> RepositoryResult<(usize, Vec<DomainOrder>)> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;

        let OrderListQuery {
            hub_id,
            status,
            customer_id,
            vendor_id,
            search,
            updated_after,
            updated_before,
            pagination,
        } = query;

        let status_filter = status.map(String::from);
        let search_pattern = search.as_ref().map(|term| format!("%{}%", term));

        let mut count_query = orders::table
            .filter(orders::hub_id.eq(hub_id.get()))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(ref status_value) = status_filter {
            count_query = count_query.filter(orders::status.eq(status_value.as_str()));
        }

        if let Some(customer) = customer_id {
            count_query = count_query.filter(orders::customer_id.eq(Some(customer.get())));
        }

        if let Some(vendor_id) = vendor_id {
            use crate::schema::vendor_order;
            let has_vendor = exists(
                vendor_order::table
                    .filter(vendor_order::order_id.eq(orders::id))
                    .filter(vendor_order::vendor_id.eq(vendor_id.get())),
            );
            count_query = count_query.filter(has_vendor);
        }

        if let Some(ref pattern) = search_pattern {
            count_query = count_query.filter(
                orders::reference
                    .like(pattern.clone())
                    .or(orders::notes.like(pattern.clone())),
            );
        }

        if let Some(updated_after) = updated_after {
            count_query = count_query.filter(orders::updated_at.ge(updated_after));
        }

        if let Some(updated_before) = updated_before {
            count_query = count_query.filter(orders::updated_at.le(updated_before));
        }

        let total = count_query.count().get_result::<i64>(&mut conn)? as usize;

        let mut items = orders::table
            .filter(orders::hub_id.eq(hub_id.get()))
            .into_boxed::<diesel::sqlite::Sqlite>();

        if let Some(ref status_value) = status_filter {
            items = items.filter(orders::status.eq(status_value.as_str()));
        }

        if let Some(customer) = customer_id {
            items = items.filter(orders::customer_id.eq(Some(customer.get())));
        }

        if let Some(vendor_id) = vendor_id {
            use crate::schema::vendor_order;
            let has_vendor = exists(
                vendor_order::table
                    .filter(vendor_order::order_id.eq(orders::id))
                    .filter(vendor_order::vendor_id.eq(vendor_id.get())),
            );
            items = items.filter(has_vendor);
        }

        if let Some(ref pattern) = search_pattern {
            items = items.filter(
                orders::reference
                    .like(pattern.clone())
                    .or(orders::notes.like(pattern.clone())),
            );
        }

        if let Some(updated_after) = updated_after {
            items = items.filter(orders::updated_at.ge(updated_after));
        }

        if let Some(updated_before) = updated_before {
            items = items.filter(orders::updated_at.le(updated_before));
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
                DomainOrder::try_from((order, products))
            })
            .collect::<Result<Vec<DomainOrder>, TypeConstraintError>>()?;

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

            Ok(DomainOrder::try_from((created, products))?)
        })
    }

    fn update_order(
        &self,
        order_id: OrderId,
        hub_id: HubId,
        updates: &DomainUpdateOrder,
    ) -> RepositoryResult<DomainOrder> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;

        conn.transaction::<DomainOrder, RepositoryError, _>(|conn| {
            let db_updates = DbUpdateOrder::from(updates);
            let order_id_raw = order_id.get();

            let target = orders::table
                .filter(orders::id.eq(order_id_raw))
                .filter(orders::hub_id.eq(hub_id.get()));

            let updated = diesel::update(target)
                .set(&db_updates)
                .get_result::<DbOrder>(conn)?;

            let products = order_products::table
                .filter(order_products::order_id.eq(order_id_raw))
                .order(order_products::id.asc())
                .load::<DbOrderProduct>(conn)?;

            Ok(DomainOrder::try_from((updated, products))?)
        })
    }

    fn update_order_product_approvals(
        &self,
        order_id: OrderId,
        hub_id: HubId,
        updates: &[OrderProductApprovalUpdate],
        new_total_cents: PriceCents,
        updated_at: NaiveDateTime,
    ) -> RepositoryResult<DomainOrder> {
        use crate::schema::{order_products, orders};

        let mut conn = self.conn()?;

        conn.transaction::<DomainOrder, RepositoryError, _>(|conn| {
            let order_id_raw = order_id.get();

            for update in updates {
                let target = order_products::table
                    .filter(order_products::order_id.eq(order_id_raw))
                    .filter(order_products::product_id.eq(update.product_id.get()));

                let affected = diesel::update(target)
                    .set((
                        order_products::approved_quantity.eq(Some(update.approved_quantity.get())),
                        order_products::price_cents.eq(update.price_cents.get()),
                        order_products::updated_at.eq(updated_at),
                    ))
                    .execute(conn)?;

                if affected == 0 {
                    return Err(RepositoryError::NotFound);
                }
            }

            let target_order = orders::table
                .filter(orders::id.eq(order_id_raw))
                .filter(orders::hub_id.eq(hub_id.get()));

            let updated_order = diesel::update(target_order)
                .set((
                    orders::total_cents.eq(new_total_cents.get()),
                    orders::updated_at.eq(updated_at),
                ))
                .get_result::<DbOrder>(conn)?;

            let products = order_products::table
                .filter(order_products::order_id.eq(order_id_raw))
                .order(order_products::id.asc())
                .load::<DbOrderProduct>(conn)?;

            Ok(DomainOrder::try_from((updated_order, products))?)
        })
    }

    fn delete_order(&self, order_id: OrderId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::orders;

        let mut conn = self.conn()?;

        let target = orders::table
            .filter(orders::id.eq(order_id.get()))
            .filter(orders::hub_id.eq(hub_id.get()));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
