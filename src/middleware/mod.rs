//! Middleware for floz — CORS, compression, tracing, and more.
//!
//! Supports both **sync** and **async** middleware in a single pipeline.
//! Static-dispatch pipeline using tuple chaining.
//! Zero overhead — fully monomorphized at compile time.

pub mod cors;
pub mod pipeline;
pub mod security;
pub mod trace;

pub mod auth;
pub mod cache;
pub mod csrf;
pub mod rate_limit;
pub mod session;

// Re-export core types
pub use auth::AuthMiddleware;
pub use cache::{CacheMiddleware, CacheRouteMap};
pub use cors::Cors;
pub use csrf::CsrfMiddleware;
pub use pipeline::{
    AsyncLayer, AsyncMiddleware, EmptyStack, FlozPipeline, Middleware, Process, Stack, SyncLayer,
};
pub use rate_limit::RateLimitMiddleware;
pub use security::SecurityHeaders;
pub use session::SessionMiddleware;
pub use trace::{RequestId, RequestTrace};
