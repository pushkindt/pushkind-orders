//! Diesel model for product tag association records.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    product_tag::{NewProductTag as DomainNewProductTag, ProductTag as DomainProductTag},
    types::{ProductId, ProductTagId, TagId, TypeConstraintError},
};

#[derive(Debug, Clone, Identifiable, Queryable, Associations, Selectable)]
#[diesel(
    table_name = crate::schema::product_tags,
    belongs_to(super::product::Product, foreign_key = product_id),
    belongs_to(super::tag::Tag, foreign_key = tag_id)
)]
pub struct ProductTag {
    pub id: i32,
    pub product_id: i32,
    pub tag_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::product_tags)]
pub struct NewProductTag {
    pub product_id: i32,
    pub tag_id: i32,
}

impl TryFrom<ProductTag> for DomainProductTag {
    type Error = TypeConstraintError;

    fn try_from(value: ProductTag) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ProductTagId::new(value.id)?,
            product_id: ProductId::new(value.product_id)?,
            tag_id: TagId::new(value.tag_id)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

impl From<&DomainNewProductTag> for NewProductTag {
    fn from(value: &DomainNewProductTag) -> Self {
        Self {
            product_id: value.product_id.get(),
            tag_id: value.tag_id.get(),
        }
    }
}
