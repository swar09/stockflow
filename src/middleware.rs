use crate::types::{Claims, UserRole};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::encode;
use jsonwebtoken::errors::Error;
use jsonwebtoken::*;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use rand_core::OsRng;
use uuid::Uuid;

pub struct testing {}

pub async fn get_jwt(id: Uuid, role: UserRole, vendor_id: Uuid) -> Result<String, Error> {
    // get the secret from dot env
    let secret = dotenv::var("JWT_SECRET_KEY").expect("UNABLE TO GET JWT SECRET");

    let my_claims = Claims {
        user: id.into(),
        vendor: vendor_id.into(),
        role,
        exp: 3600000, // unix time 1 hr
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
            match argon2.verify_password(password, &hash) {
                Ok(_) => {
                    return true;
                }
                Err(_e) => {
                    return false;
                }
            }
        }
        Err(_e) => {
            println!("INTERNAL ERROR CANT CONVER USER PASS TO HASH FOR VERIFICATION");
            return false;
        }
    }
    false
}

pub async fn verify_jwt(token: String) -> (bool, Option<String>, Option<UserRole>, Option<String>) {
    // result : bool , user_id : some UUid , User_role Some Enum, Vendor_id: soome uuid ,
    let secret = dotenv::var("JWT_SECRET_KEY").expect("UNABLE TO GET JWT SECRET");
    let result = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    );
    match result {
        Ok(tokendata) => {
            let user_id = tokendata.claims.user; // user id
            let user_role = tokendata.claims.role; // user role
            let vendor_id = tokendata.claims.vendor; // vendor id

            (true, Some(user_id), Some(user_role), Some(vendor_id))
        }
        Err(_e) => (false, None, None, None),
    }
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    // let (_, user_id, user_role, vendor_id) = ;
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
