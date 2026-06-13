-- ============================================================================
-- seed.sql
-- Seeds: vendor (10), item (10 per vendor = 100), item_variant (for ~40% of
-- items, 2-3 variants each), users (2-3 per vendor + 1 sys_admin).
--
-- Designed to be RE-RUNNABLE: it wipes the seeded tables first (in FK-safe
-- order), then re-inserts everything with gen_random_uuid().
-- ============================================================================

BEGIN;

-- Clean slate (order matters because of FKs)
TRUNCATE TABLE item_variant, item, users, vendor RESTART IDENTITY CASCADE;

DO $$
DECLARE
    vendor_names TEXT[] := ARRAY[
        'Acme Supplies', 'BlueWave Foods', 'Nordic Outdoor Gear', 'Sunrise Organics',
        'TechNova Electronics', 'Pioneer Hardware', 'Velvet Threads Apparel',
        'Coastal Coffee Co', 'Apex Auto Parts', 'Greenfield Farms Co-op'
    ];

    vendor_statuses vendor_status[] := ARRAY[
        'active','active','active','active','active',
        'suspended','suspended','pending','pending','active'
    ]::vendor_status[];

    -- 10 vendors x 10 item names each, themed per vendor
    item_categories TEXT[][] := ARRAY[
        ARRAY['Office Supplies','Stationery','Paper Goods','Printer Ink','Notebooks','Pens','Folders','Sticky Notes','Desk Organizers','Labels'],
        ARRAY['Canned Soup','Pasta Sauce','Snack Bars','Frozen Vegetables','Cereal','Cooking Oil','Spices','Rice','Beverages','Condiments'],
        ARRAY['Hiking Boots','Tents','Sleeping Bags','Backpacks','Camping Stoves','Headlamps','Trekking Poles','Rain Jackets','Water Bottles','Compasses'],
        ARRAY['Organic Honey','Granola','Herbal Tea','Almond Butter','Quinoa','Dried Fruit','Coconut Oil','Chia Seeds','Oat Milk','Trail Mix'],
        ARRAY['Bluetooth Speaker','Wireless Mouse','USB-C Cable','Power Bank','Smart Plug','Webcam','Mechanical Keyboard','Monitor Stand','Earbuds','LED Strip Light'],
        ARRAY['Cordless Drill','Hammer','Wrench Set','Tool Box','Paint Roller','Extension Cord','Work Gloves','Safety Goggles','Tape Measure','Utility Knife'],
        ARRAY['Cotton T-Shirt','Denim Jacket','Wool Scarf','Sneakers','Leather Belt','Summer Dress','Hoodie','Linen Shirt','Sun Hat','Ankle Socks'],
        ARRAY['Espresso Beans','Cold Brew Concentrate','French Press','Coffee Grinder','Travel Mug','Pour Over Kit','Dark Roast Beans','Decaf Beans','Coffee Filters','Milk Frother'],
        ARRAY['Brake Pads','Air Filter','Spark Plugs','Oil Filter','Wiper Blades','Car Battery','Headlight Bulb','Floor Mats','Tire Pressure Gauge','Car Wax'],
        ARRAY['Free Range Eggs','Raw Milk','Grass Fed Beef','Heirloom Tomatoes','Maple Syrup','Goat Cheese','Sourdough Bread','Local Honey','Pasture Butter','Seasonal Vegetables']
    ];

    units      TEXT[] := ARRAY['piece','box','kg','liter','pack','dozen','set','pair','roll','bottle'];
    currencies TEXT[] := ARRAY['USD','EUR','GBP','INR','CAD'];
    countries  TEXT[] := ARRAY['USA','China','Germany','India','Vietnam','Italy','Mexico'];
    regions    TEXT[] := ARRAY['North America','Europe','Asia Pacific','Latin America'];
    channels   TEXT[] := ARRAY['referral','marketplace','sales-team','self-signup'];

    item_statuses item_status[] := ARRAY['active','active','active','inactive','archived']::item_status[];
    all_tags TEXT[] := ARRAY['new','bestseller','eco-friendly','sale','limited','imported','handmade','premium','clearance','seasonal','organic','exclusive'];
    colors   TEXT[] := ARRAY['Red','Blue','Green','Black','White','Yellow','Gray','Navy'];
    sizes    TEXT[] := ARRAY['XS','S','M','L','XL','XXL'];

    v_id    UUID;
    item_id UUID;
    i INT;
    j INT;
    k INT;
    n_tags INT;
    n_variants INT;
    has_var BOOLEAN;
    slug_base TEXT;
    item_nm TEXT;
BEGIN
    FOR i IN 1..10 LOOP
        v_id := gen_random_uuid();
        slug_base := lower(regexp_replace(vendor_names[i], '[^a-zA-Z0-9]+', '-', 'g'));

        INSERT INTO vendor (id, name, slug, status, email, metadata, created_at, updated_at)
        VALUES (
            v_id,
            vendor_names[i],
            slug_base,
            vendor_statuses[i],
            lower(regexp_replace(vendor_names[i], '[^a-zA-Z0-9]+', '', 'g')) || '@' || slug_base || '.com',
            hstore(ARRAY[
                ['contact_phone', '+1-555-0' || (1000 + i * 7)::TEXT],
                ['region', regions[1 + (i % array_length(regions, 1))]],
                ['onboarded_via', channels[1 + (i % array_length(channels, 1))]]
            ]),
            now() - (interval '1 day' * (i * 17)),
            CASE WHEN i % 3 = 0 THEN now() - (interval '1 day' * i) ELSE NULL END
        );

        -- Operator user for every vendor
        INSERT INTO users (id, vendor_id, role, name, email, passkey, created_at, updated_at)
        VALUES (
            gen_random_uuid(), v_id, 'operator',
            'Operator - ' || vendor_names[i],
            'operator@' || slug_base || '.com',
            crypt('password123', gen_salt('bf')),
            now() - (interval '1 day' * (i * 10)), NULL
        );

        -- Read-only user for even-indexed vendors
        IF i % 2 = 0 THEN
            INSERT INTO users (id, vendor_id, role, name, email, passkey, created_at, updated_at)
            VALUES (
                gen_random_uuid(), v_id, 'read_only_user',
                'Viewer - ' || vendor_names[i],
                'viewer@' || slug_base || '.com',
                crypt('viewpass', gen_salt('bf')),
                now() - (interval '1 day' * (i * 5)), NULL
            );
        END IF;

        -- Vendor admin for first three vendors
        IF i <= 3 THEN
            INSERT INTO users (id, vendor_id, role, name, email, passkey, created_at, updated_at)
            VALUES (
                gen_random_uuid(), v_id, 'admin',
                'Admin - ' || vendor_names[i],
                'admin@' || slug_base || '.com',
                crypt('adminpass', gen_salt('bf')),
                now() - (interval '1 day' * (i * 12)), now() - interval '2 days'
            );
        END IF;

        -- 10 items per vendor
        FOR j IN 1..10 LOOP
            item_id := gen_random_uuid();
            item_nm := item_categories[i][j];
            n_tags  := 1 + (random() * 3)::INT;        -- 1-4 tags
            has_var := (random() < 0.4);               -- ~40% have variants

            INSERT INTO item (
                id, vendor_id, sku, name, description, status, unit_of_measure,
                base_price, currency_code, category_ids, tags, attributes,
                image_urls, has_variants, created_at, updated_at
            ) VALUES (
                item_id,
                v_id,
                upper(substring(slug_base from 1 for 3)) || '-' || lpad(j::TEXT, 4, '0'),
                item_nm,
                item_nm || ' supplied by ' || vendor_names[i] || '. Carefully sourced, quality checked, and ready to ship.',
                item_statuses[1 + ((i + j) % array_length(item_statuses, 1))],
                units[1 + ((i + j) % array_length(units, 1))],
                500 + ((i * j * 137) % 9500),
                currencies[1 + ((i + j) % array_length(currencies, 1))],
                ARRAY(SELECT gen_random_uuid() FROM generate_series(1, 1 + (j % 3))),
                (SELECT array_agg(all_tags[1 + ((i + j + x) % array_length(all_tags, 1))])
                   FROM generate_series(1, n_tags) AS x),
                hstore(ARRAY[
                    ['weight_kg', round((0.1 + (j * 0.37))::numeric, 2)::TEXT],
                    ['country_of_origin', countries[1 + ((i + j) % array_length(countries, 1))]],
                    ['warranty_months', ((i + j) % 24)::TEXT]
                ]),
                ARRAY[
                    'https://images.example.com/' || slug_base || '/' || j || '_1.jpg',
                    'https://images.example.com/' || slug_base || '/' || j || '_2.jpg'
                ],
                has_var,
                now() - (interval '1 hour' * (i * 100 + j)),
                CASE WHEN (i + j) % 4 = 0 THEN now() - (interval '1 hour' * j) ELSE NULL END
            );

            IF has_var THEN
                n_variants := 2 + (random() * 1)::INT; -- 2-3 variants
                FOR k IN 1..n_variants LOOP
                    INSERT INTO item_variant (
                        id, item_id, vendor_id, sku, name, status, option_values,
                        base_price, attributes, image_urls, created_at, updated_at
                    ) VALUES (
                        gen_random_uuid(),
                        item_id,
                        v_id,
                        upper(substring(slug_base from 1 for 3)) || '-' || lpad(j::TEXT, 4, '0') || '-V' || k,
                        item_nm || ' - ' || colors[1 + ((i + j + k) % array_length(colors, 1))]
                                 || ' / ' || sizes[1 + ((i + j + k) % array_length(sizes, 1))],
                        item_statuses[1 + ((i + j + k) % array_length(item_statuses, 1))],
                        hstore(ARRAY[
                            ['color', colors[1 + ((i + j + k) % array_length(colors, 1))]],
                            ['size', sizes[1 + ((i + j + k) % array_length(sizes, 1))]]
                        ]),
                        500 + ((i * j * k * 97) % 9500),
                        hstore(ARRAY[
                            ['sku_origin', 'variant'],
                            ['stock_qty', ((i + j + k) * 3 % 200)::TEXT]
                        ]),
                        ARRAY['https://images.example.com/' || slug_base || '/' || j || '_v' || k || '.jpg'],
                        now() - (interval '1 hour' * (i * 50 + j * 5 + k)),
                        NULL
                    );
                END LOOP;
            END IF;
        END LOOP;
    END LOOP;

    -- Global sys admin, not tied to any vendor
    INSERT INTO users (id, vendor_id, role, name, email, passkey, created_at, updated_at)
    VALUES (
        gen_random_uuid(), NULL, 'sys_admin', 'System Administrator', 'sysadmin@platform.internal',
        crypt('SuperSecret!2026', gen_salt('bf')), now() - interval '90 days', NULL
    );

    -- Global API service account
    INSERT INTO users (id, vendor_id, role, name, email, passkey, created_at, updated_at)
    VALUES (
        gen_random_uuid(), NULL, 'service', 'Internal API Service', 'svc-api@platform.internal',
        crypt(encode(gen_random_bytes(24), 'hex'), gen_salt('bf')), now() - interval '60 days', NULL
    );

    RAISE NOTICE 'Seed data inserted successfully.';
END $$;

COMMIT;

-- Quick sanity counts
SELECT 'vendor' AS table_name, count(*) FROM vendor
UNION ALL
SELECT 'item', count(*) FROM item
UNION ALL
SELECT 'item_variant', count(*) FROM item_variant
UNION ALL
SELECT 'users', count(*) FROM users;