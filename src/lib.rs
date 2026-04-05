//! # floz
//!
//! A batteries-included MVC web framework for Rust.
//! Built on ntex + Floz — convention over configuration, like Django/Rails for Rust.
//!
//! ## Quick Start
//!
//! ```ignore
//! use floz::prelude::*;
//!
//! #[route(get: "/health", tag: "System", desc: "Health check")]
//! async fn health() -> HttpResponse {
//!     HttpResponse::Ok().body("OK")
//! }
//!
//! #[route(
//!     get: "/users/:id",
//!     tag: "Users",
//!     desc: "Get user by ID",
//!     resps: [(200, "User found"), (404, "Not found")],
//! )]
//! async fn get_user(path: web::types::Path<i32>) -> HttpResponse {
//!     let id = path.into_inner();
//!     HttpResponse::Ok().json(&json!({ "id": id, "name": "Alice" }))
//! }
//!
//! #[floz::main]
//! async fn main() -> std::io::Result<()> {
//!     App::new().run().await   // auto-discovers all #[route] handlers
//! }
//! ```

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Core modules — always available
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub use {chrono, serde_json};

pub mod app;
pub mod config;
pub mod errors;
pub mod server;
pub mod db;
pub mod controller;
pub mod macros;
pub mod router;

// Re-export external crates used by macros
pub use inventory;
#[doc(hidden)]
pub use ntex;
pub use utoipa;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Framework Aliases & Wrappers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The floz async entry point
pub use floz_macros::main;

/// Web constructs (Path, Json, Request/Response, etc.)
pub mod web {
    pub use ntex::web::{
        self,
        middleware,
        types::{Path, Json, Query, Payload, State},
        Error, HttpRequest, HttpResponse, HttpResponseBuilder
    };
    pub use ntex::http::{header, StatusCode};
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Feature-gated modules
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub mod middleware;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "worker")]
pub mod worker;

#[cfg(feature = "worker")]
pub mod cache;

#[cfg(feature = "logger")]
pub mod logger;

#[cfg(feature = "openapi")]
pub mod openapi;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Re-exports for convenience
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub use floz_orm::*;

pub mod prelude;

