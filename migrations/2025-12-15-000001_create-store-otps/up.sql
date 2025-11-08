CREATE TABLE store_otps (
    hub_id INTEGER NOT NULL,
    phone TEXT NOT NULL,
    code TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    last_sent_at TIMESTAMP NOT NULL,
    PRIMARY KEY (hub_id, phone)
);
