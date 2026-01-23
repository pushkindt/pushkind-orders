-- Your SQL goes here
CREATE TABLE vendors (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    hub_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(hub_id, name)
);
CREATE TABLE vendor_user (
    vendor_id INTEGER NOT NULL REFERENCES vendors(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(vendor_id, user_id)
);
CREATE TABLE vendor_order (
    vendor_id INTEGER NOT NULL REFERENCES vendors(id) ON DELETE CASCADE,
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    UNIQUE(vendor_id, order_id)
);
ALTER TABLE products ADD COLUMN vendor_id INTEGER REFERENCES vendors(id) ON DELETE SET NULL;
CREATE INDEX idx_vendors_hub_id_name ON vendors(hub_id, name);
CREATE INDEX idx_vendors_hub_id_id ON vendors(hub_id, id);
CREATE UNIQUE INDEX idx_vendor_user_user_id ON vendor_user(user_id);
CREATE UNIQUE INDEX idx_vendor_order_order_id ON vendor_order(order_id);
