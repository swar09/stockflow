use crate::types::Vendor;
use crate::types::VendorHandlerResponse;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub pass: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    login: bool,
    bearer: String, // JWT
    expires_at: DateTime<Utc>,
}
pub async fn login_handler(_payload: Json<LoginPayload>) -> Json<LoginResponse> {
    // some fuctionn which will verifiy payload and call jwt middleware

    
    Json(LoginResponse {
        login: false,
        bearer: String::from("Testing"),
        expires_at: Utc::now(),
    })
}

pub async fn vendor_handler(_payload: Json<Vendor>) -> Json<VendorHandlerResponse> {
    Json(VendorHandlerResponse {})
}
