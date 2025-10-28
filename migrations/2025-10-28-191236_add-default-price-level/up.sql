-- Your SQL goes here
ALTER TABLE price_levels ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT 1;
