//! JWT token management.
//!
//! Provides token creation, verification, and claims management.
//! Extracted from auth/logic/jwt_token_management.rs.

use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use crate::errors::ApiError;

/// JWT Claims payload.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// Role
    pub role: String,
    /// Audience
    pub aud: String,
    /// Issuer
    pub iss: String,
}

/// Create a JWT token for the given user.
pub fn create_token(
    user_id: &str,
    role: &str,
    secret: &[u8],
    audience: &str,
    issuer: &str,
    expiry_hours: u64,
) -> Result<(String, u64), ApiError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let expiry_seconds = expiry_hours * 3600;
    let exp = now + expiry_seconds as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        iat: now,
        role: role.to_string(),
        aud: audience.to_string(),
        iss: issuer.to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?;

    Ok((token, expiry_seconds))
}

/// Verify and decode a JWT token.
pub fn verify_token(
    token: &str,
    secret: &[u8],
    audience: &str,
    issuer: &str,
) -> Result<Claims, ApiError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[audience]);
    validation.set_issuer(&[issuer]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation,
    )?;

    Ok(token_data.claims)
}
