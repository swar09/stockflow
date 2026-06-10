use jsonwebtoken::encode;
use jsonwebtoken::errors::Error;
use serde::Deserialize;
use serde::Serialize;
pub struct testing {}
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // Subject (usually user ID)
    role: String, // Custom claim
    exp: usize,   // Expiration time (Required for validation)
}
use jsonwebtoken::*;

pub async fn get_jwt() -> Result<String, Error> {
    let secret = b"some-dagerous-passkey";
    let my_claims = Claims {
        sub: String::from("user123"),
        role: String::from("somehthing"),
        exp: 3600000, // unix time 1 hr
    };

    let token = encode(
        &Header::default(),
        &my_claims,
        &EncodingKey::from_secret(secret),
    );

    match token {
        Ok(token) => Ok(token),
        Err(e) => Err(e),
    }
}
