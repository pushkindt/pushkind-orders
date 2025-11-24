-- Your SQL goes here
CREATE VIRTUAL TABLE product_fts USING fts5(
    name,
    sku,
    description,
    content='products',
    content_rowid='id',
    tokenize = 'unicode61'
);

INSERT INTO product_fts(product_fts) VALUES('rebuild');

CREATE TRIGGER products_ai AFTER INSERT ON products BEGIN
  INSERT INTO product_fts(rowid, name, sku, description) VALUES (new.id, new.name, new.sku, new.description);
END;
CREATE TRIGGER products_ad AFTER DELETE ON products BEGIN
  INSERT INTO product_fts(product_fts, rowid, name, sku, description) VALUES('delete', old.id, old.name, old.sku, old.description);
END;
CREATE TRIGGER products_au AFTER UPDATE ON products BEGIN
  INSERT INTO product_fts(product_fts, rowid, name, sku, description) VALUES('delete', old.id, old.name, old.sku, old.description);
  INSERT INTO product_fts(rowid, name, sku, description) VALUES (new.id, new.name, new.sku, new.description);
END;
