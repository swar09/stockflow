-- Add migration script here
CREATE TYPE vendor_status AS ENUM ('active','suspended','pending');
CREATE TABLE vendor (
    id UUID PRIMARY KEY gen_random_uuid() ,
    name VARCHAR(100) NOT NUll,
    slug VARCHAR(100),
    status vendor_status NOT NUll,
    email VARCHAR(100) NOT NUll,
    metadata hstore , 
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
);

CREATE TYPE item_status AS ENUM ('active', 'inactive', 'archived');
CREATE TABLE item (
    id UUID PRIMARY KEY gen_random_uuid(), 
    vendor_id UUID NOT NULL,
    sku VARCHAR(100) NOT NULL,
    name VARCHAR(100) NOT NULL, 
    description VARCHAR(500), 
    status item_status NOT NULL,
    unit_of_measure VARCHAR(80), 
    base_price INT,
    currency_code VARCHAR(10),
    category_ids UUID[], 
    tags TEXT[],
    attributes hstore,
    image_urls TEXT[],
    has_variants BOOLEAN,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT fk_vendor_item FOREIGN KEY(vendor_id) REFERENCES vendor(id)
);

CREATE TABLE item_variant (
    id UUID PRIMARY KEY gen_random_uuid(),
    item_id UUID NOT NULL, 
    vendor_id UUID NOT NULL, 
    sku VARCHAR(100) NOT NULL,
    name VARCHAR(100) NOT NULL,
    status item_status NOT NULL,
    option_values hstore NOT NULL, 
    base_price INT , 
    attributes hstore,
    image_urls TEXT[],
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT fk_vendor_item_variant FOREIGN KEY(vendor_id) REFERENCES vendor(id)
);
    
    
    
    -- id UUID PRIMARY KEY gen_random_uuid(), 
    -- vendor_id UUID NOT NULL,