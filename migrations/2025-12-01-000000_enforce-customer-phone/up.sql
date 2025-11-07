PRAGMA foreign_keys = OFF;

DROP INDEX IF EXISTS customers_hub_email_phone_idx;
DROP INDEX IF EXISTS customers_hub_id_email_idx;
DROP INDEX IF EXISTS customers_hub_id_phone_idx;

DELETE FROM customers WHERE phone IS NULL OR trim(phone) = '';

ALTER TABLE customers RENAME COLUMN phone TO phone_old;
ALTER TABLE customers ADD COLUMN phone TEXT NOT NULL DEFAULT '';
UPDATE customers SET phone = phone_old;
ALTER TABLE customers DROP COLUMN phone_old;

ALTER TABLE customers RENAME COLUMN email TO email_old;
ALTER TABLE customers ADD COLUMN email TEXT;
UPDATE customers SET email = email_old;
ALTER TABLE customers DROP COLUMN email_old;

CREATE UNIQUE INDEX customers_hub_id_email_idx
    ON customers(hub_id, email);
CREATE UNIQUE INDEX customers_hub_id_phone_idx
    ON customers(hub_id, phone);
CREATE INDEX IF NOT EXISTS customers_hub_id_idx ON customers(hub_id);
CREATE INDEX IF NOT EXISTS customers_price_level_id_idx ON customers(price_level_id);

PRAGMA foreign_keys = ON;
