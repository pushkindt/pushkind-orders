-- This file should undo anything in `up.sql`
ALTER TABLE order_products DROP COLUMN default_price_cents;
