CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX users_hub_id_email_idx ON users(hub_id, email);
CREATE INDEX users_hub_id_idx ON users(hub_id);

CREATE TABLE products (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    sku TEXT,
    description TEXT,
    price_cents INTEGER NOT NULL,
    currency TEXT NOT NULL,
    is_archived BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX products_hub_id_sku_idx
    ON products(hub_id, sku)
    WHERE sku IS NOT NULL;
CREATE INDEX products_hub_id_idx ON products(hub_id);
CREATE INDEX products_is_archived_idx ON products(is_archived);

CREATE TABLE orders (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    customer_id INTEGER,
    reference TEXT,
    status TEXT NOT NULL,
    notes TEXT,
    total_cents INTEGER NOT NULL,
    currency TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        status IN (
            'Draft',
            'Pending',
            'Processing',
            'Completed',
            'Cancelled'
        )
    )
);

CREATE UNIQUE INDEX orders_hub_id_reference_idx
    ON orders(hub_id, reference)
    WHERE reference IS NOT NULL;
CREATE INDEX orders_hub_id_idx ON orders(hub_id);
CREATE INDEX orders_status_idx ON orders(status);
