ALTER TABLE customers
    ADD COLUMN phone TEXT;

DROP INDEX IF EXISTS customers_hub_id_email_idx;

CREATE UNIQUE INDEX IF NOT EXISTS customers_hub_email_phone_idx
    ON customers(hub_id, email, phone);
