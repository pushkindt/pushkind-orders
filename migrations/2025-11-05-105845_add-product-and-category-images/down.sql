-- This file should undo anything in `up.sql`
ALTER TABLE categories DROP COLUMN image_url;
DROP TABLE product_images;
