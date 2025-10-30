ALTER TABLE customers
    ADD COLUMN phone TEXT;

CREATE INDEX IF NOT EXISTS customers_hub_email_phone_idx
    ON customers(hub_id, email, phone);
