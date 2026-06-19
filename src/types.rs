use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct Vendor {
    pub id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub status: Status,
    pub email: String,
    #[sqlx(json)]
    pub metadata: Option<HashMap<String, String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct CsvRecordVendor {
    pub slug: Option<String>,
    pub name: String,
    pub status: Status,
    pub email: String,
    #[sqlx(json)]
    pub metadata: Option<HashMap<String, String>>,
    #[sqlx(json)]
    pub items: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Item {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Itemstatus,
    pub base_price: Option<i32>,
    pub currency_code: Option<String>,
    #[sqlx(rename = "category_ids")]
    pub catgeory_ids: Option<Vec<Uuid>>,
    #[sqlx(skip)]
    #[serde(default)]
    pub units: i32,
    #[sqlx(skip)]
    #[serde(default)]
    pub variants: Option<Vec<Uuid>>,
    #[sqlx(skip)]
    #[serde(default)]
    pub stock: i32,
    #[sqlx(rename = "unit_of_measure")]
    pub uom: Option<String>,
    pub tags: Option<Vec<String>>,
    #[sqlx(json)]
    pub attributes: Option<HashMap<String, String>>,
    pub image_urls: Option<Vec<String>>,
    pub has_variants: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CsvRecordItem {
    pub vendor_id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Itemstatus,
    pub base_price: Option<i32>,
    pub currency_code: Option<String>,
    pub catgeory_ids: Option<Vec<Uuid>>,
    pub units: i32,
    pub variants: Option<Vec<Uuid>>,
    pub stock: i32,
    pub uom: Option<String>,
    pub tags: Option<Vec<String>>,
    #[sqlx(json)]
    pub attributes: Option<HashMap<String, String>>,
    pub image_urls: Option<Vec<String>>,
    pub has_variants: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ItemVariant {
    pub id: Uuid,
    pub item_id: Uuid,
    pub vendor_id: Uuid,
    pub sku: String,
    pub name: String,
    pub status: Itemstatus,
    #[sqlx(json)]
    pub option_values: HashMap<String, String>,
    pub base_price: i32,
    #[sqlx(json)]
    pub attributes: HashMap<String, String>,
    #[sqlx(skip)]
    #[serde(default)]
    pub stock: i32,
    pub image_urls: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CsvRecordItemVariant {
    pub id: Uuid,
    pub item_id: Uuid,
    pub vendor_id: Uuid,
    pub sku: String,
    pub name: String,
    pub status: Itemstatus,
    pub option_values: HashMap<String, String>,
    pub base_price: i32,
    pub attributes: HashMap<String, String>,
    pub stock: i32,
    pub image_urls: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct Category {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<Uuid>,
    pub description: Option<String>,
    pub sort_order: i32,
    #[sqlx(json)]
    pub attributes: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct CategoryPayload {
    pub vendor_id: Uuid,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<Uuid>,
    pub description: Option<String>,
    pub sort_order: i32,
    #[sqlx(json)]
    pub attributes: Option<HashMap<String, String>>,
}

pub struct CsvRecordCatgeory {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<Uuid>,
    pub description: String,
    pub sort_order: i8,
    pub attributes: HashMap<String, String>,
    pub created_at: Utc,
    pub updated_at: Utc,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct StockRecord {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub item_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub location: String,
    pub quantity_on_hand: i32,
    pub quantity_reserved: i32,
    pub quantity_available: i32,
    pub reorder_point: i32,
    pub reorder_quantity: i32,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct StockAdjustment {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub stock_record_info: Uuid,
    pub adjustment_type: AdjustmenType,
    pub quantity_delta: i32,
    pub quantity_before: i32,
    pub quantity_after: i32,
    pub reason: String,
    pub reference_id: String,
    pub performed_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct ApiKey {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub status: ApiStatus,
    pub last_used_time: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub vendor_id: Option<Uuid>,
    pub role: UserRole,
    pub email: String,
    pub passkey: String,
}

#[derive(Serialize)]
pub struct VendorHandlerResponse {}

#[derive(Deserialize, Clone)]
pub enum Role {
    Admin,
    Operator,
    ReadOnly,
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "api_status", rename_all = "lowercase")]
pub enum ApiStatus {
    #[serde(rename = "Active")]
    Active,
    #[serde(rename = "Revoked")]
    Revoked,
}

#[derive(Deserialize, Debug, Clone, sqlx::Type, Serialize, PartialEq)]
#[sqlx(type_name = "adjustment_type", rename_all = "lowercase")]
pub enum AdjustmenType {
    Default,
}

#[derive(Deserialize, Debug, Clone, sqlx::Type, Serialize, PartialEq)]
#[sqlx(type_name = "vendor_status", rename_all = "lowercase")]
pub enum Status {
    #[serde(rename = "Active")]
    Active,
    #[serde(rename = "Suspened")]
    #[sqlx(rename = "suspended")]
    Suspened,
    #[serde(rename = "Pedning")]
    #[sqlx(rename = "pending")]
    Pedning,
}

#[derive(Deserialize, Debug, Clone, sqlx::Type, Serialize, PartialEq)]
#[sqlx(type_name = "item_status", rename_all = "lowercase")]
pub enum Itemstatus {
    #[serde(rename = "Active")]
    Active,
    #[serde(rename = "Inactive")]
    Inactive,
    #[serde(rename = "Archived")]
    Archived,
}

#[derive(PartialEq, Deserialize, Debug, Clone, sqlx::Type, Serialize, Eq, PartialOrd, Ord)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    #[serde(rename = "Read_Only_User")]
    #[sqlx(rename = "read_only_user")]
    Read_Only_User,
    #[serde(rename = "Operator")]
    #[sqlx(rename = "operator")]
    Operator,
    #[serde(rename = "Service")]
    #[sqlx(rename = "service")]
    Service,
    #[serde(rename = "Admin")]
    #[sqlx(rename = "admin")]
    Admin,
    #[serde(rename = "Api")]
    #[sqlx(rename = "api")]
    Api,
    #[serde(rename = "Sys_Admin")]
    #[sqlx(rename = "sys_admin")]
    Sys_Admin,
}

#[derive(Deserialize, Serialize, FromRow, Debug, Clone)]
pub struct ItemPayload {
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Itemstatus,
    pub base_price: Option<i32>,
    pub currency_code: Option<String>,
    pub catgeory_ids: Option<Vec<Uuid>>,
    pub units: i32,
    pub variants: Option<Vec<ItemVariant>>,
    pub stock: i32,
    pub uom: Option<String>,
    pub tags: Option<Vec<String>>,
    pub attributes: Option<HashMap<String, String>>,
    pub image_urls: Option<Vec<String>>,
    pub has_variants: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user: String,
    pub vendor: String,
    pub role: UserRole,
    pub exp: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoginPayload {
    pub email: String,
    pub pass: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SignupPayload {
    pub name: String,
    pub email: String,
    pub pass: String,
    pub role: UserRole,
    pub vendor_id: Option<Uuid>,
}

#[derive(Serialize, Debug, Clone)]
pub struct LoginResponse {
    pub login: bool,
    pub bearer: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SignupResponse {
    pub result: bool,
    pub message: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApiPayload {
    pub vendor_id: Uuid,
    pub name: String,
}
