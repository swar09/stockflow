use crate::types::{ApiStatus, Category, Claims, UserRole};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::encode;
use jsonwebtoken::errors::Error;
use jsonwebtoken::*;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use rand_core::OsRng;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_jwt(id: Uuid, role: UserRole, vendor_id: Option<Uuid>) -> Result<String, Error> {
    let secret = dotenv::var("JWT_SECRET_KEY").expect("UNABLE TO GET JWT SECRET");

    let my_claims = Claims {
        user: id.to_string(),
        vendor: vendor_id.map(|v| v.to_string()).unwrap_or_default(),
        role,
        exp: (chrono::Utc::now().timestamp() + 3600000) as usize,
    };

    let token = encode(
        &Header::default(),
        &my_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    );

    match token {
        Ok(token) => Ok(token),
        Err(e) => Err(e),
    }
}

pub async fn get_pass_key(pass: String) -> String {
    let password = pass.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password, &salt).unwrap().to_string();
    let parsed_hash = PasswordHash::new(&password_hash).unwrap().to_string();
    parsed_hash
}

pub async fn verify_passkey(hashed: String, password: &[u8]) -> bool {
    let result = PasswordHash::new(&hashed);
    match result {
        Ok(hash) => {
            let argon2 = Argon2::default();
            argon2.verify_password(password, &hash).is_ok()
        }
        Err(_) => {
            println!("INTERNAL ERROR CANT CONVERT USER PASS TO HASH FOR VERIFICATION");
            false
        }
    }
}

pub async fn verify_jwt(token: String) -> (bool, Option<String>, Option<UserRole>, Option<String>) {
    let secret = dotenv::var("JWT_SECRET_KEY").expect("UNABLE TO GET JWT SECRET");
    let result = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    );
    match result {
        Ok(tokendata) => {
            let user_id = tokendata.claims.user;
            let user_role = tokendata.claims.role;
            let vendor_id = tokendata.claims.vendor;

            (true, Some(user_id), Some(user_role), Some(vendor_id))
        }
        Err(_) => (false, None, None, None),
    }
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match verify_jwt(String::from(token)).await {
        (_, Some(user_id), Some(user_role), Some(vendor_id)) => {
            req.extensions_mut().insert(Claims {
                user: user_id,
                vendor: vendor_id,
                role: user_role,
                exp: 3600000,
            });
        }
        _ => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(req).await)
}

pub async fn check_vendor_id(vendor_id: String, claims_id: String) -> bool {
    vendor_id == claims_id
}

pub async fn verify_api_key(id: Uuid, hash: String, State(pool): State<PgPool>) -> bool {
    let result: Option<ApiStatus> =
        sqlx::query_scalar("SELECT api_status FROM apikey WHERE id = $1 AND key_hash = $2")
            .bind(id)
            .bind(hash)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
    matches!(result, Some(ApiStatus::Active))
}

pub async fn get_cat_by_cat_id(id: Uuid, State(pool): State<PgPool>) -> Option<Category> {
    let result = sqlx::query_as::<_, Category>(
        "SELECT id, vendor_id, name, slug, parent_id, description, sort_order, hstore_to_jsonb(attributes) as attributes, created_at, updated_at 
         FROM category WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&pool)
    .await;

    result.ok()
}
