-- Your SQL goes here
ALTER TABLE categories ADD COLUMN image_url TEXT;
CREATE TABLE product_images (
    id INTEGER NOT NULL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    image_url TEXT NOT NULL
);
