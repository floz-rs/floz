#![cfg(feature = "auth")]

use floz::auth::{api_key, jwt};

#[test]
fn test_api_key_generation() {
    let key = api_key::generate_api_key("xf");
    assert!(key.starts_with("xf_"));
    // prefix (3) + nanoid (21) = 24
    assert_eq!(key.len(), 24);
}

#[test]
fn test_api_key_hashing_and_verification() {
    let key = "my_secret_api_key";
    let hash = api_key::hash_api_key(key).expect("Failed to hash api key");
    
    // Hash should not be the plaintext key
    assert_ne!(key, hash);
    
    // Valid verify
    let is_valid = api_key::verify_api_key_hash(key, &hash).expect("Verification failed");
    assert!(is_valid, "Key should match hash");
    
    // Invalid verify
    let is_invalid = api_key::verify_api_key_hash("wrong_key", &hash).expect("Verification failed");
    assert!(!is_invalid, "Wrong key should not match hash");
}

#[test]
fn test_jwt_creation_and_verification() {
    let secret = b"super_secret_key_for_testing";
    let audience = "test_audience";
    let issuer = "test_issuer";
    
    // Create token
    let (token, expiry) = jwt::create_token(
        "user_123",
        "admin",
        secret,
        audience,
        issuer,
        24 // 24 hours
    ).expect("Failed to create token");
    
    assert!(token.len() > 0);
    assert_eq!(expiry, 24 * 3600);
    
    // Verify token
    let claims = jwt::verify_token(&token, secret, audience, issuer).expect("Failed to verify token");
    
    assert_eq!(claims.sub, "user_123");
    assert_eq!(claims.role, "admin");
    assert_eq!(claims.aud, audience);
    assert_eq!(claims.iss, issuer);
}

#[test]
fn test_jwt_verification_failure() {
    let secret = b"super_secret_key_for_testing";
    let audience = "test_audience";
    let issuer = "test_issuer";
    
    let (token, _) = jwt::create_token(
        "user_123",
        "admin",
        secret,
        audience,
        issuer,
        24 // 24 hours
    ).expect("Failed to create token");
    
    // Try with wrong secret
    let wrong_secret = b"wrong_secret";
    let err = jwt::verify_token(&token, wrong_secret, audience, issuer);
    assert!(err.is_err(), "Should fail with wrong secret");
    
    // Try with wrong audience
    let err = jwt::verify_token(&token, secret, "wrong_audience", issuer);
    assert!(err.is_err(), "Should fail with wrong audience");
    
    // Try with wrong issuer
    let err = jwt::verify_token(&token, secret, audience, "wrong_issuer");
    assert!(err.is_err(), "Should fail with wrong issuer");
}
