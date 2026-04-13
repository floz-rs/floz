//! Authentication module for floz.
//!
//! Provides JWT token management, API key authentication,
//! cookie-based auth, and captcha/honeypot validation.
//!
//! Enabled via the `auth` feature flag (included in `full`).

pub mod api_key;
pub mod jwt;
