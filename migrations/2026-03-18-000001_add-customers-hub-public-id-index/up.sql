CREATE UNIQUE INDEX IF NOT EXISTS customers_hub_id_public_id_idx
    ON customers (hub_id, public_id);

DROP TABLE IF EXISTS store_otps;
