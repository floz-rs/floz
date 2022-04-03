//! HTTP request logging middleware.
//!
//! Logs method, path, status, and duration for every request.
//!
//! # Example
//! ```ignore
//! use floz::prelude::*;
//!
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(HttpLogger)
//!     )
//!     .run()
//!     .await
//! ```

use crate::middleware::Middleware;
use ntex::web::{HttpRequest, HttpResponse};

/// HTTP request logger middleware.
///
/// Logs `→ METHOD /path` on request and `← STATUS (Xms)` on response.
#[derive(Clone)]
pub struct HttpLogger;

impl Middleware for HttpLogger {
    fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        tracing::info!("→ {} {}", req.method(), req.path());
        None // always continue
    }

    fn response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        tracing::info!("← {}", resp.status().as_u16());
        resp
    }

    fn name(&self) -> &str {
        "http_logger"
    }
}
