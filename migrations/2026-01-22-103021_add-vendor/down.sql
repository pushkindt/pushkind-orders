-- This file should undo anything in `up.sql`
ALTER TABLE products DROP COLUMN vendor_id;
DROP INDEX IF EXISTS idx_vendors_hub_id_id;
DROP INDEX IF EXISTS idx_vendors_hub_id_name;
DROP TABLE IF EXISTS vendor_user;
DROP TABLE IF EXISTS vendor_order;
DROP TABLE IF EXISTS vendors;
