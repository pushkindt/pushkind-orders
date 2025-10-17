DROP INDEX IF EXISTS product_tags_tag_id_idx;
DROP INDEX IF EXISTS product_tags_product_id_idx;
DROP INDEX IF EXISTS product_tags_product_id_tag_id_idx;
DROP TABLE IF EXISTS product_tags;

DROP INDEX IF EXISTS tags_hub_id_idx;
DROP INDEX IF EXISTS tags_hub_id_name_idx;
DROP TABLE IF EXISTS tags;
