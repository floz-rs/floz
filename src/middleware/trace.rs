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
use ntex::http::header::{HeaderName, HeaderValue};

/// Type containing the strongly-typed Request ID generated/extracted from the HTTP pipeline.
///
/// Can be extracted by handlers natively if needed: `req.extensions().get::<RequestId>()`
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// Request tracing middleware.
///
/// - `handle()`: logs incoming request with method + path + request ID
/// - `response()`: logs outgoing response with status code + request ID
///   - 5xx → `error!`
///   - 4xx → `warn!`
///   - 2xx/3xx → `info!`
///
/// Automatically generates an `X-Request-Id` using a UUID if the incoming request 
/// does not supply one. The ID will be echoed back into the response header.
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
        let req_id = req.headers().get("x-request-id")
            .and_then(|h| h.to_str().ok().map(|s| s.to_string()))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            
        req.extensions_mut().insert(RequestId(req_id.clone()));

        if self.include_headers {
            tracing::debug!(
                request_id = %req_id,
                method = %req.method(),
                path = %req.path(),
                version = ?req.version(),
                headers = ?req.headers(),
                "→ request"
            );
        } else {
            tracing::info!(
                request_id = %req_id,
                method = %req.method(),
                path = %req.path(),
                "→ request"
            );
        }
        None // always continue
    }

    fn response(&self, req: &HttpRequest, mut resp: HttpResponse) -> HttpResponse {
        let status = resp.status().as_u16();
        let req_id = req.extensions().get::<RequestId>().map(|r| r.0.clone()).unwrap_or_else(|| "-".to_string());

        // Echo it back to the client
        if let Ok(header_val) = HeaderValue::from_str(&req_id) {
            resp.headers_mut().insert(HeaderName::from_static("x-request-id"), header_val);
        }

        if status >= 500 {
            tracing::error!(
                request_id = %req_id,
                method = %req.method(),
                path = %req.path(),
                status,
                "← server error"
            );
        } else if status >= 400 {
            tracing::warn!(
                request_id = %req_id,
                method = %req.method(),
                path = %req.path(),
                status,
                "← client error"
            );
        } else {
            tracing::info!(
                request_id = %req_id,
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
