use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
// use std::os::OsRng;
// use rand_os::OsRng;
// use argon2::password_hash::rand_os::OsRng;
use rand_core::OsRng;

use jsonwebtoken::encode;
use jsonwebtoken::errors::Error;
use jsonwebtoken::*;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::types::UserRole;

pub struct testing {}
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    user: String,   // Subject (usually user ID)
    vendor: String, // Custom claim
    role: UserRole,
    exp: usize, // Expiration time (Required for validation)
}

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
