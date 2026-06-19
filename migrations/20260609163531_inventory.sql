CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE EXTENSION IF NOT EXISTS hstore;

CREATE TYPE vendor_status AS ENUM ('active', 'suspended', 'pending');

CREATE TABLE vendor (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100),
    status vendor_status NOT NULL,
    email VARCHAR(100) NOT NULL,
    metadata hstore,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE TYPE item_status AS ENUM ('active', 'inactive', 'archived');

CREATE TABLE item (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendor_id UUID NOT NULL,
    sku VARCHAR(100) NOT NULL,
    name VARCHAR(100) NOT NULL,
    description VARCHAR(500),
    status item_status NOT NULL,
    unit_of_measure VARCHAR(80),
    base_price INT,
    currency_code VARCHAR(10),
    category_ids UUID [],
    tags TEXT [],
    attributes hstore,
    image_urls TEXT [],
    has_variants BOOLEAN,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_vendor_item FOREIGN KEY (vendor_id) REFERENCES vendor(id),
    CONSTRAINT UNIQUE_id_item UNIQUE (vendor_id, sku)
);

CREATE TABLE item_variant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id UUID NOT NULL,
    vendor_id UUID NOT NULL,
    sku VARCHAR(100) NOT NULL,
    name VARCHAR(100) NOT NULL,
    status item_status NOT NULL,
    option_values hstore NOT NULL,
    base_price INT,
    attributes hstore,
    image_urls TEXT [],
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_vendor_item_variant FOREIGN KEY (vendor_id) REFERENCES vendor(id)
);

CREATE  TABLE stock ();

CREATE TYPE user_status AS ENUM ('active', 'suspended', 'pending');

CREATE TYPE user_role AS ENUM (
    'admin',
    'api',
    'operator',
    'read_only_user',
    'service',
    'sys_admin'
);

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendor_id UUID,
    -- status user_status NOT NULL,
    role user_role NOT NULL,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    passkey VARCHAR,
    -- this is cryptic hash
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_vendor_user FOREIGN KEY (vendor_id) REFERENCES vendor(id)
);


CREATE OR REPLACE FUNCTION update_modified_column() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_vendor_modtime BEFORE
UPDATE
    ON vendor FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_item_modtime BEFORE
UPDATE
    ON item FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_users_modtime BEFORE
UPDATE
    ON users FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_item_variant_modtime BEFORE
UPDATE
    ON item_variant FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TYPE api_status AS ENUM ('active', 'revoked');

CREATE TABLE category (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendor_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    parent_id UUID,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    attributes hstore,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_vendor_category FOREIGN KEY (vendor_id) REFERENCES vendor(id)
);

CREATE TABLE stockrecord (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendor_id UUID NOT NULL,
    item_id UUID NOT NULL,
    variant_id UUID,
    location VARCHAR(100) NOT NULL,
    quantity_on_hand INT NOT NULL DEFAULT 0,
    quantity_reserved INT NOT NULL DEFAULT 0,
    quantity_available INT NOT NULL DEFAULT 0,
    reorder_point INT NOT NULL DEFAULT 0,
    reorder_quantity INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_vendor_stockrecord FOREIGN KEY (vendor_id) REFERENCES vendor(id),
    CONSTRAINT fk_item_stockrecord FOREIGN KEY (item_id) REFERENCES item(id),
    CONSTRAINT fk_variant_stockrecord FOREIGN KEY (variant_id) REFERENCES item_variant(id)
);

CREATE TABLE apikey (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendor_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    key_prefix VARCHAR(100) NOT NULL,
    key_hash VARCHAR(255) NOT NULL,
    status api_status NOT NULL,
    api_status api_status NOT NULL,
    last_used_time TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_vendor_apikey FOREIGN KEY (vendor_id) REFERENCES vendor(id)
);

CREATE TRIGGER update_category_modtime BEFORE
UPDATE
    ON category FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_stockrecord_modtime BEFORE
UPDATE
    ON stockrecord FOR EACH ROW EXECUTE FUNCTION update_modified_column();