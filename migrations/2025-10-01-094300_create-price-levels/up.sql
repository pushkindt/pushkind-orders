CREATE TABLE price_levels (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX price_levels_hub_id_name_idx
    ON price_levels(hub_id, name);
CREATE INDEX price_levels_hub_id_idx ON price_levels(hub_id);
