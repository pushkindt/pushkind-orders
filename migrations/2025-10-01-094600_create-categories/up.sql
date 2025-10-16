CREATE TABLE categories (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    parent_id INTEGER,
    name TEXT NOT NULL,
    description TEXT,
    is_archived BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX categories_hub_id_parent_id_name_idx
    ON categories(hub_id, parent_id, name);
CREATE INDEX categories_hub_id_idx ON categories(hub_id);
CREATE INDEX categories_parent_id_idx ON categories(parent_id);
CREATE INDEX categories_is_archived_idx ON categories(is_archived);
