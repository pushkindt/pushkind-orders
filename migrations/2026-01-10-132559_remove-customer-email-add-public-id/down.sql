ALTER TABLE customers DROP COLUMN public_id;
ALTER TABLE customers ADD COLUMN email TEXT;

CREATE UNIQUE INDEX customers_hub_id_email_idx
    ON customers(hub_id, email);
