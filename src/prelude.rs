//! Convenience re-exports for common floz usage.
//!
//! ```ignore
//! use floz::prelude::*;
//! ```

// Core app
pub use crate::app::{App, AppContext, Context, RequestContext, AuthInfo};
pub use crate::config::Config;
pub use crate::errors::{ApiError, ErrorCode};

/// Shared framework state — the one way to access db, config, and all
/// shared services in a `#[route]` handler.
///
/// ```ignore
/// #[route(get: "/notes")]
/// async fn list_notes(state: State) -> HttpResponse {
///     let notes = Note::all(&state.db()).await?;
///     HttpResponse::Ok().json(&notes)
/// }
/// ```
pub type State = crate::web::State<AppContext>;

// Database
#[cfg(feature = "postgres")]
pub use crate::db::{PgDbPool, pg_pool};
#[cfg(feature = "sqlite")]
pub use crate::db::{SqliteDbPool, sqlite_pool};
// Legacy alias when only postgres is enabled
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub use crate::db::{DbPool, pool};

// Controller
pub use crate::controller::pagination::PaginationParams;
pub use crate::controller::format::JsonResponse;
pub use crate::controller::validate::ValidationErrors;

// Server
pub use crate::server::ServerConfig;

// Middleware
pub use crate::middleware::Middleware;
pub use crate::middleware::AsyncMiddleware;
pub use crate::middleware::CacheMiddleware;
pub use crate::middleware::SessionMiddleware;
pub use crate::middleware::AuthMiddleware;
pub use crate::middleware::cors::Cors;
pub use crate::middleware::trace::{RequestTrace, RequestId};
pub use crate::middleware::SecurityHeaders;
pub use crate::middleware::RateLimitMiddleware;
pub use crate::middleware::CsrfMiddleware;


#[cfg(feature = "logger")]
pub use crate::logger::HttpLogger;

// ORM — tight integration
pub use floz_orm::prelude::*;

// Cache
#[cfg(feature = "worker")]
pub use crate::cache::Cache;

// Macros
pub use crate::{echo, res, pp, xquery};

// Web Sub-modules (WS & Upload)
pub use crate::web;

// Route macro — the `#[route(...)]` attribute
pub use floz_macros::route;

// Model macro — the `#[model("table")]` attribute
pub use floz_macros::model;

// Task macro
#[cfg(feature = "worker")]
pub use floz_macros::task;

// Channel Gate macro
pub use floz_macros::channel_gate;

// Framework HTTP & Web constructs
pub use crate::main;
// pub use crate::web::{self, HttpResponse, HttpRequest};

// Type aliases for extractors so IDEs show `floz::prelude::Path` instead of `ntex::...`
pub type Path<T> = crate::web::Path<T>;
pub type Json<T> = crate::web::Json<T>;
pub type Query<T> = crate::web::Query<T>;
pub type Payload = crate::web::Payload;

/// Short alias for HTTP responses in route handlers.
pub type Resp = crate::web::HttpResponse;
pub type Req = crate::web::HttpRequest;

pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};

pub use tracing::{info, warn, error, debug, trace};
pub use floz_macros::embed_migrations;
