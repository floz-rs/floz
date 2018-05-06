//! Error types for floz.
//!
//! Provides `ApiError` and `ErrorCode` with automatic conversion from
//! common error types (sqlx, serde_json, JWT, UUID, Redis, anyhow).

mod api_error;

pub use api_error::{ApiError, ErrorCode};
