use crate::{
    middleware::{
        check_vendor_id, get_cat_by_cat_id, get_jwt, get_pass_key, verify_jwt, verify_passkey,
    },
    types::{
        Category, CategoryPayload, Claims, CsvRecordItem, CsvRecordVendor, Item, ItemPayload,
        ItemVariant, Itemstatus, StockAdjustment, StockRecord, User, UserRole, Vendor,
    },
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub pass: String,
}

#[derive(Deserialize)]
pub struct SignupPayload {
    pub name: String,
    pub email: String,
    pub pass: String,
    pub role: UserRole,
    // pub vendor: Uuid,
}

#[derive(Serialize)]
pub struct LoginResponse {
    login: bool,
    bearer: String,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct SignupResponse {
    // login: bool,
    // bearer: String,
    // expires_at: DateTime<Utc>,
    result: bool,
    message: String,
}
// use std::result::Result::Ok;
pub async fn signup_handler(
    State(pool): State<PgPool>,
    payload: Json<SignupPayload>,
) -> Result<Json<SignupResponse>, StatusCode> {
    // query the db and write to users table
    // and return the result
    let result = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
        .bind(&payload.email)
        .fetch_one(&pool)
        .await;
    match result {
        Ok(true) => {
            return Err(StatusCode::CONFLICT);
        }
        Ok(false) => { /*Do nothing user doesnot exists*/ }
        Err(_e) => {}
    }
    let passkey = get_pass_key(payload.pass.clone()).await;
    let result =
        sqlx::query("INSERT INTO users (name, email, passkey, role) VALUES ($1,$2,$3,$4);")
            .bind(&payload.name)
            .bind(&payload.email)
            .bind(passkey)
            .bind(&payload.role)
            .execute(&pool)
            .await;
    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                Ok(Json(SignupResponse {
                    result: true,
                    message: String::from("Signup Sucessfull !"),
                }))
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn login_handler(
    State(pool): State<PgPool>,
    payload: Json<LoginPayload>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user_result = sqlx::query_as::<_, User>(
        "SELECT id, vendor_id, role, passkey FROM users where email = $1",
    )
    .bind(&payload.email)
    .fetch_one(&pool)
    .await;

    match user_result {
        Ok(u) => {
            if verify_passkey(u.passkey, payload.pass.as_bytes()).await {
                match get_jwt(u.id, u.role, u.vendor_id).await {
                    Ok(token) => Ok(Json(LoginResponse {
                        login: true,
                        bearer: token,
                        expires_at: Some(Utc::now()),
                    })),
                    Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            } else {
                // println!("wrong password");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_vendors(State(pool): State<PgPool>) -> Result<Json<Vec<Vendor>>, StatusCode> {
    let token = String::from("");
    let (_, _, user_role, _) = verify_jwt(token).await;
    if user_role.unwrap() == UserRole::Sys_Admin {
        let result = sqlx::query_as::<_, Vendor>("SELECT * from vendors ;")
            .fetch_all(&pool)
            .await;

        match result {
            Ok(vendor) => Ok(Json(vendor)),
            Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn delete_vendor(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(id): Json<Uuid>,
) -> Result<Json<Vendor>, StatusCode> {
    if claims.role < UserRole::Admin
        && claims.vendor != id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result =
        sqlx::query_as::<_, Vendor>("UPDATE vendor SET status = 'suspended' WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await;

    match result {
        Ok(vendor) => Ok(Json(vendor)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn put_vendor(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<Vendor>,
) -> Result<Json<Vendor>, StatusCode> {
    if claims.role < UserRole::Admin
        && claims.vendor != payload.id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = sqlx::query_as::<_, Vendor>(
        "UPDATE vendors SET 
        name = $1, 
        email = $2, 
        metadata = $3 
        WHERE id = $4
        RETURNING *",
    )
    .bind(payload.name)
    .bind(payload.email)
    .bind(sqlx::types::Json(payload.metadata))
    .bind(payload.id)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(vendor) => Ok(Json(vendor)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_vendor_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
) -> Result<Json<Vendor>, StatusCode> {
    if claims.role < UserRole::Admin
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query_as::<_, Vendor>("SELECT * FROM vendor WHERE id = $1")
        .bind(vendor_id)
        .fetch_one(&pool)
        .await;

    match result {
        Ok(vendor) => Ok(Json(vendor)),
        Err(_e) => {
            // error handling db error
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn add_new_item(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
    payload: Json<ItemPayload>,
) -> Result<Json<bool>, StatusCode> {
    if claims.role < UserRole::Operator
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query("INSERT INTO item (vendor_id, sku, name, description, status, base_price, currency_code, catgeory_ids,  unit_of_measure, variant, has_variants, tags, attributes, image_urls ) 
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14); ")
    .bind(vendor_id)
    .bind(&payload.sku)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.base_price)
    .bind(&payload.currency_code)
    .bind(&payload.catgeory_ids)
    .bind(&payload.uom)
    .bind(sqlx::types::Json(&payload.variants))
    .bind(payload.has_variants)
    .bind(&payload.tags)
    .bind(sqlx::types::Json(&payload.attributes))
    .bind(&payload.image_urls)
    .execute(&pool).await;
    match result {
        Ok(_query) => Ok(Json(true)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_item_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Item>, StatusCode> {
    if claims.role < UserRole::Read_Only_User {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result =
        sqlx::query_as::<_, Item>("SELECT  * from item WHERE vendor_id = $1 AND item_id = $2")
            .bind(vendor_id)
            .bind(item_id)
            .fetch_one(&pool)
            .await;
    match result {
        Ok(item) => Ok(Json(item)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn put_item_by_id(
    State(pool): State<PgPool>,
    Path(vendor_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    payload: Json<ItemPayload>,
) -> Result<Json<bool>, StatusCode> {
    if claims.role < UserRole::Operator
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query("INSERT INTO item (vendor_id, sku, name, description, status, base_price, currency_code, catgeory_ids,  unit_of_measure, variant, has_variants, tags, attributes, image_urls ) 
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14); ")
    .bind(vendor_id)
    .bind(&payload.sku)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.base_price)
    .bind(&payload.currency_code)
    .bind(&payload.catgeory_ids)
    .bind(&payload.uom)
    .bind(sqlx::types::Json(&payload.variants))
    .bind(payload.has_variants)
    .bind(&payload.tags)
    .bind(sqlx::types::Json(&payload.attributes))
    .bind(&payload.image_urls)
    .execute(&pool).await;
    match result {
        Ok(_query) => Ok(Json(true)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_items_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
) -> Result<Json<Option<Vec<Item>>>, StatusCode> {
    if claims.role < UserRole::Operator
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query_as::<_, Item>(
        "SELECT * FROM item WHERE vendor_id = $1 AND item_status != 'archived'",
    )
    .bind(vendor_id)
    .fetch_all(&pool)
    .await;
    match result {
        Ok(item) => Ok(Json(Some(item))),
        Err(_e) => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn archive_item_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<bool>, StatusCode> {
    if claims.role < UserRole::Admin
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query("UPDATE item SET status = $1 WHERE item_id = $2 AND vendor_id = $3")
        .bind(Itemstatus::Archived)
        .bind(item_id)
        .bind(vendor_id)
        .execute(&pool)
        .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                Ok(Json(true))
            } else {
                Err(StatusCode::NO_CONTENT)
            }
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn set_sku_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(sku): Json<String>,
) -> Result<Json<Option<(bool, String, String)>>, StatusCode> {
    if claims.role < UserRole::Operator
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query("UPDATE item SET sku = $1 WHERE vendor_id = $2 AND item_id = $3")
        .bind(&sku)
        .bind(vendor_id)
        .bind(item_id)
        .execute(&pool)
        .await;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                Ok(Json(Some((true, item_id.to_string(), sku))))
            } else {
                Err(StatusCode::NO_CONTENT)
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_skus_by_id(
    State(pool): State<PgPool>,
    Path(vendor_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<(String,)>>, StatusCode> {
    if claims.role < UserRole::Read_Only_User {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let result = sqlx::query_as::<_, (String,)>("SELECT sku FROM item WHERE vendor_id = $1")
        .bind(vendor_id)
        .fetch_all(&pool)
        .await;
    match result {
        Ok(vec) => Ok(Json(vec)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn post_csv_vendors(
    State(pool): State<PgPool>,
    Json(payload): Json<Vec<CsvRecordVendor>>,
) -> Result<Json<bool>, StatusCode> {
    if let Some(record) = payload.into_iter().next() {
        let result = sqlx::query(
            "INSERT INTO vendor (slug , name, status, email, metadata, items)
            VALUES ($1 ,$2 ,$3 ,$4 ,$5 ,$6 )",
        )
        .bind(record.slug)
        .bind(record.name)
        .bind(record.status)
        .bind(record.email)
        .bind(sqlx::types::Json(record.metadata))
        .bind(sqlx::types::Json(record.items))
        .execute(&pool)
        .await;

        match result {
            Ok(query) => {
                if query.rows_affected() == 0 {
                    return Err(StatusCode::CONFLICT);
                } else {
                    return Ok(Json(true));
                }
            }
            Err(_e) => {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }
    Err(StatusCode::NOT_MODIFIED)
}
pub async fn post_csv_items(
    State(pool): State<PgPool>,
    Json(payload): Json<Vec<CsvRecordItem>>,
) -> Result<Json<bool>, StatusCode> {
    if let Some(record) = payload.into_iter().next() {
        let result = sqlx::query(
            "INSERT INTO vendor (vendor_id , sku, name, description, status, base_price, currency_code, catgeory_ids, units, variants, stock, uom, tags, attributes, image_urls, has_variant)
            VALUES ($1 ,$2 ,$3 ,$4 ,$5 ,$6, $7 ,$8 ,$9 ,$10 ,$11 ,$12 ,$13 ,$14 ,$15 ,$16)",
        )
        .bind(record.vendor_id)
        .bind(record.sku)
        .bind(record.name)
        .bind(record.description)
        .bind(record.status)
        .bind(record.base_price)
        .bind(record.currency_code)
        .bind(record.catgeory_ids)
        .bind(record.units)
        .bind(record.variants)
        .bind(record.stock)
        .bind(record.uom)
        .bind(record.tags)
        .bind(sqlx::types::Json(record.attributes))
        .bind(sqlx::types::Json(record.image_urls))
        .bind(record.has_variants)
        .execute(&pool)
        .await;

        match result {
            Ok(query) => {
                if query.rows_affected() == 0 {
                    return Err(StatusCode::NO_CONTENT);
                } else {
                    return Ok(Json(true));
                }
            }
            Err(_e) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
    Err(StatusCode::NOT_MODIFIED)
}

pub async fn get_variant_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<ItemVariant>>, StatusCode> {
    if claims.role < UserRole::Operator
        && claims.vendor != vendor_id.to_string()
        && claims.role != UserRole::Sys_Admin
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result =
        sqlx::query_scalar("SELECT variant FROM item WHERE vendor_id = $1 AND item_id = $2")
            .bind("vendor_id")
            .bind("item_id")
            .fetch_one(&pool)
            .await;
    let mut uuids: Vec<Uuid> = Vec::new();
    match result {
        Ok(variant) => match variant {
            None => return Err(StatusCode::NOT_FOUND),
            Some(vec) => {
                uuids = vec;
            }
        },
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
    let mut variants: Vec<ItemVariant> = Vec::new();
    for id in uuids {
        let result = sqlx::query_as::<_, ItemVariant>("SELECT * FROM item_variant WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await;
        match result {
            Ok(variant) => {
                variants.push(variant);
            }
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    Ok(Json(variants))
}

pub async fn put_variant_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id, variant_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ItemVariant>,
) -> Result<Json<bool>, StatusCode> {
    match claims.role {
        UserRole::Read_Only_User => {
            return Err(StatusCode::UNAUTHORIZED);
        }
        UserRole::Admin => {}
        _ => {
            if !(check_vendor_id(vendor_id.to_string(), claims.vendor).await) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    let result = sqlx::query("INSERT INTO item_variant (item_id,vendor_id,sku, name, option_values, base_price, attributes, stock, image_urls) VALUES ($2,$3,$4,$5,$6,$7,$8,$9) WHERE id = $1")
    .bind(variant_id)
    .bind(item_id)
    .bind(vendor_id)
    .bind(payload.sku)
    .bind(payload.name)
    .bind(payload.status)
    .bind(sqlx::types::Json(payload.option_values))
    .bind(payload.base_price)
    .bind(sqlx::types::Json(payload.attributes))
    .bind(payload.stock)
    .bind(sqlx::types::Json(payload.image_urls))
    .execute(&pool)
    .await;

    match result {
        Ok(query) => {
            if query.rows_affected() == 0 {
                Err(StatusCode::NOT_MODIFIED)
            } else {
                Ok(Json(true))
            }
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_cats_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Category>>, StatusCode> {
    if claims.role < UserRole::Read_Only_User {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = sqlx::query_scalar::<_, Vec<Uuid>>(
        "SELECT catgeory_ids FROM item WHERE vendor_id = $1 AND item_id = $2",
    )
    .bind(vendor_id)
    .bind(item_id)
    .fetch_one(&pool)
    .await;
    let mut cat_ids = Vec::new();
    match result {
        Ok(vec) => {
            cat_ids = vec;
        }
        Err(_e) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    let mut cats = Vec::new();
    for id in cat_ids {
        match get_cat_by_cat_id(id, State(pool.clone())).await {
            Some(cat) => {
                cats.push(cat);
            }
            None => {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(Json(cats))
}

pub async fn get_cat_by_id(
    State(pool): State<PgPool>,
    Json(id): Json<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Category>, StatusCode> {
    if claims.role < UserRole::Read_Only_User {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match get_cat_by_cat_id(id, State(pool.clone())).await {
        Some(c) => Ok(Json(c)),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn put_cat_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id, category_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CategoryPayload>,
) -> Result<Json<bool>, StatusCode> {
    match claims.role {
        UserRole::Read_Only_User => return Err(StatusCode::UNAUTHORIZED),
        UserRole::Sys_Admin => {}
        _ => {
            if claims.vendor != vendor_id.to_string() {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    let result = sqlx::query("UPDATE category vendor_id = $1 , name = $2, slug = $3, parent_id = $4, description = $4, sort_order = $5, attributes = $6 WHERE id = $7 ")
    .bind(payload.vendor_id)
    .bind(payload.name)
    .bind(payload.slug)
    .bind(payload.parent_id)
    .bind(payload.description   )
    .bind(payload.sort_order)
    .bind(sqlx::types::Json(payload.attributes))
    .bind(category_id)
    .execute(&pool).await    ;

    match result {
        Ok(query) => {
            if query.rows_affected() != 0 {
                Err(StatusCode::NOT_MODIFIED)
            } else {
                Ok(Json(true))
            }
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn delete_cat_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id, category_id)): Path<(Uuid, Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<bool>, StatusCode> {
    match claims.role {
        UserRole::Read_Only_User => return Err(StatusCode::UNAUTHORIZED),
        UserRole::Sys_Admin => {}
        _ => {
            if claims.vendor != vendor_id.to_string() {
                return Err(StatusCode::UNAUTHORIZED);
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
                Ok(Json(true))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_stock_record_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, item_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<StockRecord>, StatusCode> {
    if vendor_id.to_string() != claims.vendor  {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = sqlx::query_as("SELECT * FROM stockrecord WHERE vendor_id = $1 AND item_id = $2")
        .bind(vendor_id)
        .bind(item_id)
        .fetch_one(&pool)
        .await;
    match result {
        Ok(stockrecord) => Ok(Json(stockrecord)),
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_stock_by_id(
    State(pool): State<PgPool>,
    Path((vendor_id, _item_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<StockAdjustment>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<bool>, StatusCode> {
    match claims.role {
        UserRole::Read_Only_User => return Err(StatusCode::UNAUTHORIZED),
        UserRole::Sys_Admin => {}
        _ => {
            if claims.vendor != vendor_id.to_string() {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    let _result = sqlx::query("UPDATE TABLE stockrecord () VALUES () WHERE id = $1 ")
        .bind(payload.id)
        .execute(&pool)
        .await;

    Err(StatusCode::NOT_IMPLEMENTED)
}

pub async fn delete_stock_by_id() {}

pub async fn get_api_key() {}
pub async fn delete_api_key() {}
