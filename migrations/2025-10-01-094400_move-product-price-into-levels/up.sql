ALTER TABLE products DROP COLUMN price_cents;

CREATE TABLE product_price_levels (
    id INTEGER NOT NULL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    price_level_id INTEGER NOT NULL REFERENCES price_levels(id) ON DELETE CASCADE,
    price_cents INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(product_id, price_level_id)
);

CREATE INDEX product_price_levels_product_id_idx
    ON product_price_levels(product_id);
CREATE INDEX product_price_levels_price_level_id_idx
    ON product_price_levels(price_level_id);
