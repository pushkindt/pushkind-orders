-- Your SQL goes here
DROP INDEX customers_hub_id_email_idx;
ALTER TABLE customers DROP COLUMN email;
ALTER TABLE customers ADD COLUMN public_id TEXT;
