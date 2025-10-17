ALTER TABLE products
    ADD COLUMN category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL;

CREATE INDEX products_category_id_idx ON products(category_id);
