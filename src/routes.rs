use std::path::PathBuf;
use std::ptr::read;
use std::sync::Arc;

use crate::middleware::get_jwt;
use crate::middleware::get_pass_key;
use crate::middleware::verify_passkey;
use crate::types::User;
use crate::types::UserRole;
use crate::types::Vendor;
use crate::types::VendorHandlerResponse;
use axum::extract::State;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::pool;
use sqlx::PgPool;
use uuid::Uuid;
// use sqlx::Pool;

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

pub async fn signup_handler(
    State(pool): State<PgPool>,
    payload: Json<SignupPayload>,
) -> Json<SignupResponse> {
    // query the db and write to users table
    // and return the result
    let result = sqlx::query("SELECT 1 FROM users WHERE email = $1")
        .bind(&payload.email)
        .execute(&pool)
        .await;
    match result {
        Ok(reponse) => {
            println!("{:#?}", reponse);
            if reponse.rows_affected() != 0 {
                return Json(SignupResponse {
                    result: false,
                    message: String::from("User with same email exsits"),
                });
            }
        }
        Err(e) => {
            println!("{:#?}", e)
        }
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
        Ok(response) => {
            if response.rows_affected() == 0 {
                Json(SignupResponse {
                    result: false,
                    message: String::from("Failed"),
                })
            } else {
                Json(SignupResponse {
                    result: true,
                    message: String::from("Sucess"),
                })
            }
        }
        Err(e) => {
            println!("ERROR :{:#?}", e);
            Json(SignupResponse {
                result: false,
                message: String::from("Failed"),
            })
        }
    }

    // Json(SignupResponse {
    //     result: false,
    //     message: String::from("Sign up failed"),
    // })
}

pub async fn login_handler(
    State(pool): State<PgPool>,
    payload: Json<LoginPayload>,
) -> Json<Option<LoginResponse>> {
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
                    Ok(token) => Json(Some(LoginResponse {
                        login: true,
                        bearer: token,
                        expires_at: Some(Utc::now()),
                    })),
                    Err(_e) => Json(Some(LoginResponse {
                        login: false,
                        bearer: String::from("Failed to get_jwt"),
                        expires_at: None,
                    })),
                }
            } else {
                println!("wrong password");
                Json(None)
            }
        }
        Err(e) => {
            println!("ERROR AT LOGIN : {e}");
            Json(None)
        }
    }
}

pub async fn vendor_handler(_payload: Json<Vendor>) -> Json<VendorHandlerResponse> {
    Json(VendorHandlerResponse {})
}

pub async fn get_vendor(State(pool): State<PgPool>, Json(id): Json<Uuid>) -> Json<Option<Vendor>> {
    let result = sqlx::query_as::<_, Vendor>("SELECT * from vendors WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await;

    match result {
        Ok(vendor) => {
            return Json(Some(vendor));
        }
        Err(e) => {
            return Json(None);
        }
    }
}

pub async fn delete_vendor(
    State(pool): State<PgPool>,
    Json(id): Json<Uuid>,
) -> Json<Option<Vendor>> {
    // check jwt && role >= ADMIN
    // then only vendor suspensetion

    let result =
        sqlx::query_as::<_, Vendor>("UPDATE vendor SET status = 'suspended' WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await;

    match result {
        Ok(vendor) => {
            return Json(Some(vendor));
        }
        Err(e) => {
            return Json(None);
        }
    }
}

pub async fn put_vendor(
    State(pool): State<PgPool>,
    Json(payload): Json<Vendor>,
) -> Json<Option<Vendor>> {
    // check jwt && role >= ADMIN
    // then only vendor update

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
        Ok(vendor) => {
            return Json(Some(vendor));
        }
        Err(e) => {
            return Json(None);
        }
    }
}

