ALTER TABLE products ADD COLUMN price_cents INTEGER NOT NULL DEFAULT 0;
DROP INDEX product_price_levels_product_id_idx;
DROP INDEX product_price_levels_price_level_id_idx;
DROP TABLE product_price_levels;
