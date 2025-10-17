CREATE TABLE tags (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX tags_hub_id_name_idx ON tags(hub_id, name);
CREATE INDEX tags_hub_id_idx ON tags(hub_id);

CREATE TABLE product_tags (
    id INTEGER NOT NULL PRIMARY KEY,
    product_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX product_tags_product_id_tag_id_idx
    ON product_tags(product_id, tag_id);
CREATE INDEX product_tags_product_id_idx ON product_tags(product_id);
CREATE INDEX product_tags_tag_id_idx ON product_tags(tag_id);
