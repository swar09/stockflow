use std::collections::HashMap;
use chrono::Utc;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;


#[derive(Deserialize)]
#[derive(FromRow, Debug)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: Uuid,
    pub vendor_id: Uuid, // check weather i can estabilsh connection between vendor and item in database
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub base_price: Option<u32>,
    pub currency_code: Option<String>,
    pub catgeory_ids: Option<Vec<Uuid>>,
    pub units: u32,
    pub variants: Option<Vec<ItemVariant>>,
    pub stock: u32,
    pub uom: Option<String>, // unit of measure
    pub tags: Option<Vec<String>>,
    pub attributes: Option<HashMap<String, String>>,
    pub image_urls: Option<Vec<String>>,
    pub has_variants: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemVariant {
    pub id: Uuid,
    pub item_id: Uuid,
    pub vendor_id: Uuid,
    pub sku: String,
    pub name: String,
    pub status: Status,
    pub option_values: HashMap<String, String>,
    pub base_price: u32,
    pub attributes: HashMap<String, String>,
    pub stock: u32,
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
    pub sort_order: u8,
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
    pub quantity_on_hand: u32,
    pub quantity_reserved: u32,
    pub quantity_available: u32,
    pub reorder_point: u32,
    pub reorder_quantity: u32,
    pub updated_at: Utc,
}

pub struct StockAdjustment {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub stock_record_info: Uuid,
    pub adjustment_type: AdjustmenType,
    pub quantity_delta: u32,
    pub quantity_before: u32,
    pub quantity_after: u32,
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
    pub id : Uuid, 
    pub vendor_id : Uuid,
    pub role : UserRole,
    // pub email: 
    pub passkey : String,
}

#[derive(Serialize)]
pub struct VendorHandlerResponse {
    
}

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
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Api,
    Operator,
    Read_Only_User,
    Service,
    Sys_Admin,
}