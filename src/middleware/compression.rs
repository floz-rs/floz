//! Response compression middleware.
//!
//! Compresses response bodies using gzip when the client supports it
//! (via `Accept-Encoding` header). Only compresses buffered responses
//! (`Body::Bytes`) — streaming bodies pass through unchanged.
//!
//! # Example
//! ```ignore
//! use floz::middleware::compression::Compression;
//!
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(Compression::gzip())
//!     )
//!     .run()
//!     .await
//! ```

use crate::middleware::Middleware;
use ntex::http::header;
use ntex::web::{HttpRequest, HttpResponse};

use flate2::write::GzEncoder;
use flate2::Compression as GzLevel;
use std::io::Write;

/// Response compression middleware.
///
/// Checks `Accept-Encoding` and compresses `Body::Bytes` responses.
/// Sets `Content-Encoding: gzip` and removes `Content-Length`
/// (the compressed length differs from the original).
#[derive(Clone, Debug)]
pub struct Compression {
    /// Minimum body size in bytes to trigger compression.
    /// Bodies smaller than this are returned uncompressed.
    min_size: usize,
    /// Compression level (1-9, where 6 is default).
    level: u32,
}

impl Compression {
    /// Create a gzip compression middleware with default settings.
    /// - Min size: 1024 bytes (skip tiny responses)
    /// - Level: 6 (balanced speed/ratio)
    pub fn gzip() -> Self {
        Self {
            min_size: 1024,
            level: 6,
        }
    }

    /// Create a fast compression middleware (level 1).
    /// Best for real-time APIs where latency matters more than size.
    pub fn fast() -> Self {
        Self {
            min_size: 1024,
            level: 1,
        }
    }

    /// Set minimum body size to trigger compression.
    pub fn min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }

    /// Set compression level (1-9).
    pub fn level(mut self, level: u32) -> Self {
        self.level = level.clamp(1, 9);
        self
    }

    /// Check if the client accepts gzip encoding.
    fn client_accepts_gzip(req: &HttpRequest) -> bool {
        req.headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("gzip"))
            .unwrap_or(false)
    }

    /// Compress bytes with gzip.
    fn compress_bytes(&self, data: &[u8]) -> Option<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), GzLevel::new(self.level));
        encoder.write_all(data).ok()?;
        encoder.finish().ok()
    }
}

impl Default for Compression {
    fn default() -> Self {
        Self::gzip()
    }
}

impl Middleware for Compression {
    fn handle(&self, _req: &HttpRequest) -> Option<HttpResponse> {
        None // always continue
    }

    fn response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        // Skip if client doesn't accept gzip
        if !Self::client_accepts_gzip(req) {
            return resp;
        }

        // Skip if response already has Content-Encoding
        if resp.headers().contains_key(header::CONTENT_ENCODING) {
            return resp;
        }

        // Split response into head + body
        let (head_resp, body) = resp.into_parts();

        // Extract bytes from the body (only compress buffered responses)
        let body_inner: ntex::http::body::Body = body.into();
        match body_inner {
            ntex::http::body::Body::Bytes(ref bytes) if bytes.len() >= self.min_size => {
                if let Some(compressed) = self.compress_bytes(bytes) {
                    // Only use compressed version if it's actually smaller
                    if compressed.len() < bytes.len() {
                        let mut resp = head_resp.set_body(ntex::http::body::Body::Bytes(
                            ntex::util::Bytes::from(compressed),
                        ));
                        resp.headers_mut().insert(
                            header::CONTENT_ENCODING,
                            header::HeaderValue::from_static("gzip"),
                        );
                        resp.headers_mut().remove(header::CONTENT_LENGTH);
                        return resp;
                    }
                }
                // Compression failed or didn't help — return original
                head_resp.set_body(body_inner)
            }
            _ => {
                // Body is empty, too small, or streaming — skip compression
                head_resp.set_body(body_inner)
            }
        }
    }

    fn name(&self) -> &str {
        "compression"
    }
}
