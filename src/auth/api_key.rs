//! API Key authentication utilities.
//!
//! Provides secure API key generation and hashing.

use nanoid::nanoid;

/// Generate a new API key with the given prefix.
///
/// # Example
/// ```ignore
/// let key = generate_api_key("xf");
/// // → "xf_V1StGXR8_Z5jdHi6B-myT"
/// ```
pub fn generate_api_key(prefix: &str) -> String {
    let key = nanoid!(21);
    format!("{prefix}_{key}")
}

/// Hash an API key for secure storage using bcrypt.
pub fn hash_api_key(key: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(key, bcrypt::DEFAULT_COST)
}

/// Verify a plaintext API key against a stored hash.
pub fn verify_api_key_hash(key: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(key, hash)
}
