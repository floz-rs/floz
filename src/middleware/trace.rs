//! Request tracing middleware.
//!
//! Creates structured tracing events per request with method, path, and status.
//! Integrates with the `tracing` ecosystem for structured logging.
//!
//! # Example
//! ```ignore
//! use floz::prelude::*;
//! use floz::middleware::trace::RequestTrace;
//!
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(RequestTrace::default())
//!     )
//!     .run()
//!     .await
//! ```

use crate::middleware::Middleware;
use ntex::web::{HttpRequest, HttpResponse};

/// Request tracing middleware.
///
/// - `handle()`: logs incoming request with method + path
/// - `response()`: logs outgoing response with status code
///   - 5xx → `error!`
///   - 4xx → `warn!`
///   - 2xx/3xx → `info!`
///
/// Uses `tracing` for structured output — pairs with any
/// tracing subscriber (console, JSON, file, etc.).
#[derive(Clone, Debug)]
pub struct RequestTrace {
    /// Include request headers in trace output (DEBUG level)
    include_headers: bool,
}

impl RequestTrace {
    /// Create a new RequestTrace.
    pub fn new() -> Self {
        Self {
            include_headers: false,
        }
    }

    /// Include request headers in the trace output (DEBUG level).
    pub fn with_headers(mut self) -> Self {
        self.include_headers = true;
        self
    }
}

impl Default for RequestTrace {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for RequestTrace {
    fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        if self.include_headers {
            tracing::debug!(
                method = %req.method(),
                path = %req.path(),
                version = ?req.version(),
                headers = ?req.headers(),
                "→ request"
            );
        } else {
            tracing::info!(
                method = %req.method(),
                path = %req.path(),
                "→ request"
            );
        }
        None // always continue
    }

    fn response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        let status = resp.status().as_u16();

        if status >= 500 {
            tracing::error!(
                method = %req.method(),
                path = %req.path(),
                status,
                "← server error"
            );
        } else if status >= 400 {
            tracing::warn!(
                method = %req.method(),
                path = %req.path(),
                status,
                "← client error"
            );
        } else {
            tracing::info!(
                status,
                "← response"
            );
        }

        resp
    }

    fn name(&self) -> &str {
        "trace"
    }
}
