use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize, FromRow, Debug)]
pub struct Vendor {
    pub id: Uuid,
    pub slug: Option<String>,
    pub name: String,
    pub status: Status,
    pub email: String,
    #[sqlx(json)]
    pub metadata: Option<HashMap<String, String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(json)]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Item {
    pub id: Uuid,
    pub vendor_id: Uuid, // check weather i can estabilsh connection between vendor and item in database
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub base_price: Option<i32>,
    pub currency_code: Option<String>,
    pub catgeory_ids: Option<Vec<Uuid>>,
    pub units: i32,
    pub variants: Option<Vec<Uuid>>, // ItemVariant Uuid
    pub stock: i32,
    pub uom: Option<String>, // unit of measure
    pub tags: Option<Vec<String>>,
    #[sqlx(json)]
    pub attributes: Option<HashMap<String, String>>,
    pub image_urls: Option<Vec<String>>,
    pub has_variants: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ItemVariant {
    pub id: Uuid,
    pub item_id: Uuid,
    pub vendor_id: Uuid,
    pub sku: String,
    pub name: String,
    pub status: Status,
    pub option_values: HashMap<String, String>,
    pub base_price: i32,
    pub attributes: HashMap<String, String>,
    pub stock: i32,
    pub image_urls: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
pub struct Catgeory {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub name: String,
    pub slug: String,
    pub parent_id: Uuid,
    pub description: String,
    pub sort_order: i8,
    pub attributes: HashMap<String, String>,
    pub created_at: Utc,
    pub updated_at: Utc,
}

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
    pub updated_at: Utc,
}
pub struct StockAdjustment {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub stock_record_info: Uuid,
    pub adjustment_type: AdjustmenType,
    pub quantity_delta: i32,
    pub quantity_before: i32,
    pub quantity_after: i32,
    pub reasn: String,
    pub reference_id: String,
    pub performed_by: Uuid, // User or API key
    pub created_at: Utc,
}

pub struct ApiKey {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub role: Role,
    pub status: ApiStatus,
    pub last_used_time: Utc,
    pub expires_at: Utc,
    pub created_at: Utc,
}

#[derive(FromRow, Debug)]
pub struct User {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub role: UserRole,
    // pub email:
    pub passkey: String,
}

#[derive(Serialize)]
pub struct VendorHandlerResponse {}

#[derive(Deserialize)]
pub enum Role {
    Admin,
    Operator,
    ReadOnly,
}

#[derive(Deserialize)]
pub enum ApiStatus {
    Active,
    Revoked,
}

#[derive(Deserialize)]
pub enum AdjustmenType {
    Default,
}

#[derive(Deserialize, Debug, Clone, sqlx::Type, Serialize)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum Status {
    Active,
    Suspened,
    Pedning,
}
#[derive(Deserialize, Debug, Clone, sqlx::Type, Serialize)]
#[sqlx(type_name = "item_status", rename_all = "lowercase")]
pub enum Itemstatus {
    Active,
    Inactive,
    Archived,
}
#[derive(PartialEq, Deserialize, Debug, Clone, sqlx::Type, Serialize, Eq, PartialOrd, Ord)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Read_Only_User,
    Operator,
    Service,
    Admin,
    Api,
    Sys_Admin,
}

#[derive(Deserialize, Serialize, FromRow, Debug)]
pub struct ItemPayload {
    // pub id: Uuid,
    // pub vendor_id: Uuid, // check weather i can estabilsh connection between vendor and item in database
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub base_price: Option<i32>,
    pub currency_code: Option<String>,
    pub catgeory_ids: Option<Vec<Uuid>>,
    pub units: i32,
    pub variants: Option<Vec<ItemVariant>>,
    pub stock: i32,
    pub uom: Option<String>, // unit of measure
    pub tags: Option<Vec<String>>,
    pub attributes: Option<HashMap<String, String>>,
    pub image_urls: Option<Vec<String>>,
    pub has_variants: bool,
    // pub created_at: DateTime<Utc>,
    // pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user: String,   // Subject (usually user ID)
    pub vendor: String, // Custom claim
    pub role: UserRole,
    pub exp: usize, // Expiration time (Required for validation)
}
