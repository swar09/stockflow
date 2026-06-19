use crate::{
    middleware::{
        check_vendor_id, get_cat_by_cat_id, get_jwt, get_pass_key,
    },
    types::{
        ApiKey, ApiPayload, ApiStatus, Category, CategoryPayload, Claims, CsvRecordItem, CsvRecordVendor, Item, ItemPayload, ItemVariant, LoginPayload, LoginResponse, SignupPayload, SignupResponse, StockAdjustment, StockRecord, User, UserRole, Vendor
    },
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(skip(pool, payload))]
pub async fn signup_handler(
    State(pool): State<PgPool>,
    payload: Json<SignupPayload>,
) -> Result<Json<SignupResponse>, StatusCode> {
    tracing::info!("Starting user signup for email: {}", payload.email);
    let result = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
        .bind(&payload.email)
        .fetch_one(&pool)
        .await;
    match result {
        Ok(true) => {
            tracing::warn!("Signup failed: user already exists for email {}", payload.email);
            return Err(StatusCode::CONFLICT);
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!("Database error checking user existence: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    let passkey = get_pass_key(payload.pass.clone()).await;
    let result =
        sqlx::query("INSERT INTO users (name, email, passkey, role, vendor_id) VALUES ($1, $2, $3, $4, $5);")
            .bind(&payload.name)
            .bind(&payload.email)
            .bind(passkey)
            .bind(&payload.role)
            .bind(payload.vendor_id)
            .execute(&pool)
            .await;
    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("User signup successful for email: {}", payload.email);
                Ok(Json(SignupResponse {
                    result: true,
                    message: String::from("Signup Successful!"),
                }))
            } else {
                tracing::error!("Insert query succeeded but affected 0 rows for email {}", payload.email);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(e) => {
            tracing::error!("Database error inserting user record: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, payload))]
pub async fn login_handler(
    State(pool): State<PgPool>,
    payload: Json<LoginPayload>,
) -> Result<Json<LoginResponse>, StatusCode> {
    tracing::info!("Attempting login for email: {}", payload.email);
    let user_result = sqlx::query_as::<_, User>(
        "SELECT id, vendor_id, role, email, passkey FROM users WHERE email = $1",
    )
    .bind(&payload.email)
    .fetch_one(&pool)
    .await;

    match user_result {
        Ok(u) => {
            let passkey_bytes = payload.pass.as_bytes();
            if crate::middleware::verify_passkey(u.passkey, passkey_bytes).await {
                match get_jwt(u.id, u.role, u.vendor_id).await {
                    Ok(token) => {
                        tracing::info!("User login successful for email: {}", payload.email);
                        Ok(Json(LoginResponse {
                            login: true,
                            bearer: token,
                            expires_at: Some(Utc::now()),
                        }))
                    }
                    Err(e) => {
                        tracing::error!("JWT generation error for user {}: {:?}", payload.email, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                tracing::warn!("Login failed: invalid password for email {}", payload.email);
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Err(e) => {
            tracing::warn!("Login failed: user not found or db error for email {}: {:?}", payload.email, e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_vendors(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Vendor>>, StatusCode> {
    tracing::info!("Fetching all vendors. Requester claims role: {:?}", claims.role);
    if claims.role == UserRole::Sys_Admin {
        let result = sqlx::query_as::<_, Vendor>(
            "SELECT id, name, slug, status, email, hstore_to_jsonb(metadata) as metadata, created_at, updated_at FROM vendor;"
        )
        .fetch_all(&pool)
        .await;

        match result {
            Ok(vendor) => {
                tracing::info!("Successfully fetched {} vendors", vendor.len());
                Ok(Json(vendor))
            }
            Err(e) => {
                tracing::error!("Database error fetching all vendors: {:?}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        tracing::warn!("Unauthorized attempt to fetch all vendors by user role: {:?}", claims.role);
        Err(StatusCode::FORBIDDEN)
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn delete_vendor(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
) -> Result<Json<Vendor>, StatusCode> {
    tracing::info!("Request to suspend vendor: {}", vendor_id);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Admin || claims.vendor != vendor_id.to_string())
    {
        tracing::warn!("Unauthorized deletion request for vendor: {} by user: {}", vendor_id, claims.user);
        return Err(StatusCode::FORBIDDEN);
    }

    let result =
        sqlx::query_as::<_, Vendor>(
            "UPDATE vendor SET status = 'suspended' WHERE id = $1 
             RETURNING id, name, slug, status, email, hstore_to_jsonb(metadata) as metadata, created_at, updated_at"
        )
        .bind(vendor_id)
        .fetch_one(&pool)
        .await;

    match result {
        Ok(vendor) => {
            tracing::info!("Vendor {} successfully suspended", vendor_id);
            Ok(Json(vendor))
        }
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Database error suspending vendor {}: {:?}", vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn put_vendor(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
    Json(payload): Json<Vendor>,
) -> Result<Json<Vendor>, StatusCode> {
    tracing::info!("Request to update vendor: {}", vendor_id);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Admin || claims.vendor != vendor_id.to_string())
    {
        tracing::warn!("Unauthorized update request for vendor: {} by user: {}", vendor_id, claims.user);
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query_as::<_, Vendor>(
        "UPDATE vendor SET 
        name = $1, 
        email = $2, 
        metadata = hstore($3::jsonb) 
        WHERE id = $4
        RETURNING id, name, slug, status, email, hstore_to_jsonb(metadata) as metadata, created_at, updated_at",
    )
    .bind(payload.name)
    .bind(payload.email)
    .bind(sqlx::types::Json(payload.metadata))
    .bind(vendor_id)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(vendor) => {
            tracing::info!("Vendor {} successfully updated", vendor_id);
            Ok(Json(vendor))
        }
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Database error updating vendor {}: {:?}", vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_vendor_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
) -> Result<Json<Vendor>, StatusCode> {
    tracing::info!("Fetching vendor details for ID: {}", vendor_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string()
    {
        tracing::warn!("Unauthorized read request for vendor: {} by user: {}", vendor_id, claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query_as::<_, Vendor>(
        "SELECT id, name, slug, status, email, hstore_to_jsonb(metadata) as metadata, created_at, updated_at 
         FROM vendor WHERE id = $1"
    )
    .bind(vendor_id)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(vendor) => Ok(Json(vendor)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Database error fetching vendor {}: {:?}", vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn add_new_item(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
    Json(payload): Json<ItemPayload>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Adding new item to vendor: {} with SKU: {}", vendor_id, payload.sku);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Operator || claims.vendor != vendor_id.to_string())
    {
        tracing::warn!("Unauthorized create item request for vendor: {} by user: {}", vendor_id, claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query(
        "INSERT INTO item (vendor_id, sku, name, description, status, base_price, currency_code, category_ids, unit_of_measure, tags, attributes, image_urls, has_variants) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, hstore($11::jsonb), $12, $13);"
    )
    .bind(vendor_id)
    .bind(&payload.sku)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.base_price)
    .bind(&payload.currency_code)
    .bind(&payload.catgeory_ids)
    .bind(&payload.uom)
    .bind(&payload.tags)
    .bind(sqlx::types::Json(&payload.attributes))
    .bind(&payload.image_urls)
    .bind(payload.has_variants)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!("Item successfully created for SKU: {} under vendor: {}", payload.sku, vendor_id);
            Ok(Json(true))
        }
        Err(e) => {
            tracing::error!("Database error inserting item SKU {}: {:?}", payload.sku, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_item_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Item>, StatusCode> {
    tracing::info!("Fetching item: {} for vendor: {}", item_id, vendor_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string() {
        tracing::warn!("Unauthorized request for item details by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result =
        sqlx::query_as::<_, Item>(
            "SELECT id, vendor_id, sku, name, description, status, unit_of_measure, base_price, currency_code, category_ids, tags, hstore_to_jsonb(attributes) as attributes, image_urls, has_variants, created_at, updated_at 
             FROM item 
             WHERE vendor_id = $1 AND id = $2"
        )
        .bind(vendor_id)
        .bind(item_id)
        .fetch_one(&pool)
        .await;
    match result {
        Ok(item) => Ok(Json(item)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Database error fetching item {} under vendor {}: {:?}", item_id, vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn put_item_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ItemPayload>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Updating item: {} for vendor: {}", item_id, vendor_id);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Operator || claims.vendor != vendor_id.to_string())
    {
        tracing::warn!("Unauthorized update item request by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query(
        "UPDATE item SET 
            name = $1, 
            description = $2, 
            status = $3, 
            base_price = $4, 
            currency_code = $5, 
            category_ids = $6, 
            unit_of_measure = $7, 
            tags = $8, 
            attributes = hstore($9::jsonb), 
            image_urls = $10, 
            has_variants = $11,
            updated_at = NOW() 
         WHERE id = $12 AND vendor_id = $13"
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.base_price)
    .bind(&payload.currency_code)
    .bind(&payload.catgeory_ids)
    .bind(&payload.uom)
    .bind(&payload.tags)
    .bind(sqlx::types::Json(&payload.attributes))
    .bind(&payload.image_urls)
    .bind(payload.has_variants)
    .bind(item_id)
    .bind(vendor_id)
    .execute(&pool)
    .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("Successfully updated item {} under vendor {}", item_id, vendor_id);
                Ok(Json(true))
            } else {
                tracing::warn!("Update executed but item {} not found under vendor {}", item_id, vendor_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Database error updating item {} under vendor {}: {:?}", item_id, vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_items_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
) -> Result<Json<Option<Vec<Item>>>, StatusCode> {
    tracing::info!("Fetching all active items for vendor: {}", vendor_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string()
    {
        tracing::warn!("Unauthorized access to vendor item list by user: {}", claims.user);
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query_as::<_, Item>(
        "SELECT id, vendor_id, sku, name, description, status, unit_of_measure, base_price, currency_code, category_ids, tags, hstore_to_jsonb(attributes) as attributes, image_urls, has_variants, created_at, updated_at 
         FROM item 
         WHERE vendor_id = $1 AND status != 'archived'",
    )
    .bind(vendor_id)
    .fetch_all(&pool)
    .await;
    match result {
        Ok(item) => {
            tracing::info!("Fetched {} items for vendor {}", item.len(), vendor_id);
            Ok(Json(Some(item)))
        }
        Err(e) => {
            tracing::error!("Database error listing items for vendor {}: {:?}", vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn archive_item_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Archiving item {} for vendor {}", item_id, vendor_id);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Admin || claims.vendor != vendor_id.to_string())
    {
        tracing::warn!("Unauthorized archive item request by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query("UPDATE item SET status = 'archived' WHERE id = $1 AND vendor_id = $2")
        .bind(item_id)
        .bind(vendor_id)
        .execute(&pool)
        .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("Item {} archived successfully", item_id);
                Ok(Json(true))
            } else {
                tracing::warn!("Archive failed: Item {} not found under vendor {}", item_id, vendor_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Database error archiving item {} under vendor {}: {:?}", item_id, vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn set_sku_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(sku): Json<String>,
) -> Result<Json<Option<(bool, String, String)>>, StatusCode> {
    tracing::info!("Updating SKU of item: {} under vendor: {} to: {}", item_id, vendor_id, sku);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Operator || claims.vendor != vendor_id.to_string())
    {
        tracing::warn!("Unauthorized SKU update request by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query("UPDATE item SET sku = $1 WHERE vendor_id = $2 AND id = $3")
        .bind(&sku)
        .bind(vendor_id)
        .bind(item_id)
        .execute(&pool)
        .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("Successfully set SKU to: {} for item: {}", sku, item_id);
                Ok(Json(Some((true, item_id.to_string(), sku))))
            } else {
                tracing::warn!("SKU update failed: item {} not found for vendor {}", item_id, vendor_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Database error updating SKU of item {} under vendor {}: {:?}", item_id, vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_skus_by_id(
    State(pool): State<PgPool>,
    Path(vendor_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<(String,)>>, StatusCode> {
    tracing::info!("Listing SKUs of vendor: {}", vendor_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string() {
        tracing::warn!("Unauthorized SKU listing attempt by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query_as::<_, (String,)>("SELECT sku FROM item WHERE vendor_id = $1")
        .bind(vendor_id)
        .fetch_all(&pool)
        .await;
    match result {
        Ok(vec) => Ok(Json(vec)),
        Err(e) => {
            tracing::error!("Database error fetching SKUs for vendor {}: {:?}", vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, payload))]
pub async fn post_csv_vendors(
    State(pool): State<PgPool>,
    Json(payload): Json<Vec<CsvRecordVendor>>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Importing bulk vendors, record count: {}", payload.len());
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to begin transaction for vendor import: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    for record in payload {
        tracing::debug!("Inserting vendor record: {:?}", record.name);
        let result = sqlx::query(
            "INSERT INTO vendor (slug, name, status, email, metadata)
             VALUES ($1, $2, $3, $4, hstore($5::jsonb))",
        )
        .bind(record.slug)
        .bind(record.name)
        .bind(record.status)
        .bind(record.email)
        .bind(sqlx::types::Json(record.metadata))
        .execute(&mut *tx)
        .await;

        if let Err(e) = result {
            tracing::error!("Database error during bulk vendor insert: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit bulk vendor import: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    tracing::info!("Successfully imported bulk vendors");
    Ok(Json(true))
}

#[tracing::instrument(skip(pool, payload))]
pub async fn post_csv_items(
    State(pool): State<PgPool>,
    Json(payload): Json<Vec<CsvRecordItem>>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Importing bulk items, record count: {}", payload.len());
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to begin transaction for item import: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    for record in payload {
        let sku = record.sku.clone();
        tracing::debug!("Inserting item SKU: {}", sku);
        let result = sqlx::query(
            "INSERT INTO item (vendor_id, sku, name, description, status, base_price, currency_code, category_ids, unit_of_measure, tags, attributes, image_urls, has_variants)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, hstore($11::jsonb), $12, $13)"
        )
        .bind(record.vendor_id)
        .bind(record.sku)
        .bind(record.name)
        .bind(record.description)
        .bind(record.status)
        .bind(record.base_price)
        .bind(record.currency_code)
        .bind(record.catgeory_ids)
        .bind(record.uom)
        .bind(record.tags)
        .bind(sqlx::types::Json(record.attributes))
        .bind(record.image_urls)
        .bind(record.has_variants)
        .execute(&mut *tx)
        .await;

        if let Err(e) = result {
            tracing::error!("Database error during bulk item insert for SKU {}: {:?}", sku, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit bulk item import: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    tracing::info!("Successfully imported bulk items");
    Ok(Json(true))
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_variant_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<ItemVariant>>, StatusCode> {
    tracing::info!("Fetching variants of item {} for vendor {}", item_id, vendor_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string()
    {
        tracing::warn!("Unauthorized variant query by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query_as::<_, ItemVariant>(
        "SELECT id, item_id, vendor_id, sku, name, status, hstore_to_jsonb(option_values) as option_values, base_price, hstore_to_jsonb(attributes) as attributes, image_urls, created_at, updated_at 
         FROM item_variant 
         WHERE vendor_id = $1 AND item_id = $2"
    )
    .bind(vendor_id)
    .bind(item_id)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(variants) => {
            tracing::info!("Fetched {} variants for item {}", variants.len(), item_id);
            Ok(Json(variants))
        }
        Err(e) => {
            tracing::error!("Database error fetching variants of item {}: {:?}", item_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn put_variant_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id, variant_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ItemVariant>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Upserting variant: {} for item: {}", variant_id, item_id);
    match claims.role {
        UserRole::Read_Only_User => {
            tracing::warn!("Unauthorized variant write: Read-only user cannot update variants");
            return Err(StatusCode::FORBIDDEN);
        }
        UserRole::Admin => {}
        _ => {
            if !(check_vendor_id(vendor_id.to_string(), claims.vendor).await) {
                tracing::warn!("Unauthorized variant write: claims vendor ID mismatch");
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let result = sqlx::query(
        "INSERT INTO item_variant (id, item_id, vendor_id, sku, name, status, option_values, base_price, attributes, image_urls) 
         VALUES ($1, $2, $3, $4, $5, $6, hstore($7::jsonb), $8, hstore($9::jsonb), $10)
         ON CONFLICT (id) DO UPDATE SET 
             sku = EXCLUDED.sku, 
             name = EXCLUDED.name, 
             status = EXCLUDED.status, 
             option_values = EXCLUDED.option_values, 
             base_price = EXCLUDED.base_price, 
             attributes = EXCLUDED.attributes, 
             image_urls = EXCLUDED.image_urls,
             updated_at = NOW()"
    )
    .bind(variant_id)
    .bind(item_id)
    .bind(vendor_id)
    .bind(payload.sku)
    .bind(payload.name)
    .bind(payload.status)
    .bind(sqlx::types::Json(payload.option_values))
    .bind(payload.base_price)
    .bind(sqlx::types::Json(payload.attributes))
    .bind(payload.image_urls)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!("Variant {} successfully upserted", variant_id);
            Ok(Json(true))
        }
        Err(e) => {
            tracing::error!("Database error upserting variant {}: {:?}", variant_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_cats_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Category>>, StatusCode> {
    tracing::info!("Fetching categories for item: {}", item_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string() {
        tracing::warn!("Unauthorized categories fetch request by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query_scalar::<_, Option<Vec<Uuid>>>(
        "SELECT category_ids FROM item WHERE vendor_id = $1 AND id = $2"
    )
    .bind(vendor_id)
    .bind(item_id)
    .fetch_one(&pool)
    .await;

    let cat_ids = match result {
        Ok(Some(vec)) => vec,
        Ok(None) => return Ok(Json(Vec::new())),
        Err(sqlx::Error::RowNotFound) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Database error checking categories of item {}: {:?}", item_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut cats = Vec::new();
    for id in cat_ids {
        match get_cat_by_cat_id(id, State(pool.clone())).await {
            Some(cat) => {
                cats.push(cat);
            }
            None => {
                tracing::error!("Category ID {} listed on item {} could not be found", id, item_id);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(Json(cats))
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_cat_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((vendor_id, _item_id, category_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<Category>, StatusCode> {
    tracing::info!("Fetching category details for: {}", category_id);
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string() {
        tracing::warn!("Unauthorized read category request by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }

    match get_cat_by_cat_id(category_id, State(pool.clone())).await {
        Some(c) => Ok(Json(c)),
        None => {
            tracing::warn!("Category {} not found", category_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn put_cat_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id, category_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CategoryPayload>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Updating category: {}", category_id);
    match claims.role {
        UserRole::Read_Only_User => return Err(StatusCode::FORBIDDEN),
        UserRole::Sys_Admin => {}
        _ => {
            if claims.vendor != vendor_id.to_string() {
                tracing::warn!("Unauthorized category update request: vendor scope mismatch");
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let result = sqlx::query(
        "INSERT INTO category (id, vendor_id, name, slug, parent_id, description, sort_order, attributes)
         VALUES ($1, $2, $3, $4, $5, $6, $7, hstore($8::jsonb))
         ON CONFLICT (id) DO UPDATE SET 
             vendor_id = EXCLUDED.vendor_id, 
             name = EXCLUDED.name, 
             slug = EXCLUDED.slug, 
             parent_id = EXCLUDED.parent_id, 
             description = EXCLUDED.description, 
             sort_order = EXCLUDED.sort_order, 
             attributes = EXCLUDED.attributes,
             updated_at = NOW()"
    )
    .bind(category_id)
    .bind(payload.vendor_id)
    .bind(payload.name)
    .bind(payload.slug)
    .bind(payload.parent_id)
    .bind(payload.description)
    .bind(payload.sort_order)
    .bind(sqlx::types::Json(payload.attributes))
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!("Successfully upserted category {}", category_id);
            Ok(Json(true))
        }
        Err(e) => {
            tracing::error!("Database error upserting category {}: {:?}", category_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn delete_cat_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id, category_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Deleting category: {}", category_id);
    match claims.role {
        UserRole::Read_Only_User => return Err(StatusCode::FORBIDDEN),
        UserRole::Sys_Admin => {}
        _ => {
            if claims.vendor != vendor_id.to_string() {
                tracing::warn!("Unauthorized category delete request: vendor scope mismatch");
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let result = sqlx::query("DELETE FROM category WHERE id = $1")
        .bind(category_id)
        .execute(&pool)
        .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("Successfully deleted category {}", category_id);
                Ok(Json(true))
            } else {
                tracing::warn!("Category delete failed: Category {} not found", category_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Database error deleting category {}: {:?}", category_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn get_stock_record_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<StockRecord>, StatusCode> {
    tracing::info!("Fetching stock record for item: {} under vendor: {}", item_id, vendor_id);
    if vendor_id.to_string() != claims.vendor {
        tracing::warn!("Unauthorized stock record access by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query_as::<_, StockRecord>("SELECT * FROM stockrecord WHERE vendor_id = $1 AND item_id = $2")
        .bind(vendor_id)
        .bind(item_id)
        .fetch_one(&pool)
        .await;
    match result {
        Ok(stockrecord) => Ok(Json(stockrecord)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Database error fetching stock record for item {}: {:?}", item_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn update_stock_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<StockAdjustment>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Updating stock record ID: {} with delta: {}", payload.stock_record_info, payload.quantity_delta);
    match claims.role {
        UserRole::Read_Only_User => return Err(StatusCode::FORBIDDEN),
        UserRole::Sys_Admin => {}
        _ => {
            if claims.vendor != vendor_id.to_string() {
                tracing::warn!("Unauthorized stock adjustment: vendor scope mismatch");
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let result = sqlx::query(
        "UPDATE stockrecord 
         SET quantity_on_hand = quantity_on_hand + $1, 
             quantity_available = quantity_on_hand + $1 - quantity_reserved, 
             updated_at = NOW() 
         WHERE id = $2 AND vendor_id = $3"
    )
    .bind(payload.quantity_delta)
    .bind(payload.stock_record_info)
    .bind(vendor_id)
    .execute(&pool)
    .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("Stock record {} successfully adjusted by delta {}", payload.stock_record_info, payload.quantity_delta);
                Ok(Json(true))
            } else {
                tracing::warn!("Stock record {} not found for vendor {}", payload.stock_record_info, vendor_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Database error adjusting stock record {}: {:?}", payload.stock_record_info, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims, payload))]
pub async fn get_api_key(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ApiPayload>,
) -> Result<Json<ApiKey>, StatusCode> {
    tracing::info!("Request to generate new API key for vendor: {}", payload.vendor_id);
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Admin || claims.vendor != payload.vendor_id.to_string())
    {
        tracing::warn!("Unauthorized API key creation request by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let id = Uuid::new_v4();
    let key_prefix = format!("ak_{}", &Uuid::new_v4().to_string()[..8]);
    let key_secret = Uuid::new_v4().to_string().replace("-", "");
    let key_hash = key_secret;
    let expires_at = Utc::now() + chrono::Duration::days(365);

    let result = sqlx::query_as::<_, ApiKey>(
        "INSERT INTO apikey (id, vendor_id, name, key_prefix, key_hash, status, api_status, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, vendor_id, name, key_prefix, key_hash, status, last_used_time, expires_at, created_at"
    )
    .bind(id)
    .bind(payload.vendor_id)
    .bind(&payload.name)
    .bind(&key_prefix)
    .bind(&key_hash)
    .bind(ApiStatus::Active)
    .bind(ApiStatus::Active)
    .bind(expires_at)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(api_key) => {
            tracing::info!("Successfully created API key for vendor: {} name: {}", payload.vendor_id, payload.name);
            Ok(Json(api_key))
        }
        Err(e) => {
            tracing::error!("Database error inserting api key for vendor {}: {:?}", payload.vendor_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(skip(pool, claims))]
pub async fn delete_api_key(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(id): Json<Uuid>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Request to revoke API key ID: {}", id);

    let key_vendor_id: Option<Uuid> = sqlx::query_scalar("SELECT vendor_id FROM apikey WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    let key_vendor_id = match key_vendor_id {
        Some(v) => v,
        None => return Err(StatusCode::NOT_FOUND),
    };

    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Admin || claims.vendor != key_vendor_id.to_string())
    {
        tracing::warn!("Unauthorized API key deletion attempt by user: {}", claims.user);
        return Err(StatusCode::FORBIDDEN);
    }
    let result = sqlx::query(
        "UPDATE apikey SET status = $1, api_status = $1 WHERE id = $2"
    )
    .bind(ApiStatus::Revoked)
    .bind(id)
    .execute(&pool)
    .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                tracing::info!("API key {} successfully revoked", id);
                Ok(Json(true))
            } else {
                tracing::warn!("Revocation failed: API key {} not found", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Database error revoking API key {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
