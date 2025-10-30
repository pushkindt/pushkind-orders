DROP INDEX IF EXISTS customers_hub_email_phone_idx;

CREATE TABLE customers_tmp (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    price_level_id INTEGER REFERENCES price_levels(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO customers_tmp (id, hub_id, name, email, price_level_id, created_at, updated_at)
SELECT id, hub_id, name, email, price_level_id, created_at, updated_at
FROM customers;

DROP TABLE customers;

ALTER TABLE customers_tmp RENAME TO customers;

CREATE UNIQUE INDEX customers_hub_id_email_idx
    ON customers(hub_id, email);
CREATE INDEX customers_hub_id_idx ON customers(hub_id);
CREATE INDEX customers_price_level_id_idx ON customers(price_level_id);
