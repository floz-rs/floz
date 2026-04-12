//! Middleware for floz — CORS, compression, tracing, and more.
//!
//! Supports both **sync** and **async** middleware in a single pipeline.
//! Static-dispatch pipeline using tuple chaining.
//! Zero overhead — fully monomorphized at compile time.

pub mod cors;
pub mod pipeline;
pub mod security;
pub mod trace;

pub mod cache;
pub mod session;
pub mod auth;
pub mod rate_limit;
pub mod csrf;

// Re-export core types
pub use pipeline::{Middleware, AsyncMiddleware, Process, EmptyStack, Stack, SyncLayer, AsyncLayer, FlozPipeline};
pub use cors::Cors;
pub use trace::{RequestTrace, RequestId};
pub use cache::{CacheMiddleware, CacheRouteMap};
pub use session::SessionMiddleware;
pub use auth::AuthMiddleware;
pub use security::SecurityHeaders;
pub use rate_limit::RateLimitMiddleware;
pub use csrf::CsrfMiddleware;
