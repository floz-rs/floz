//! Security response headers middleware.
//!
//! Automatically injects standard HTTP security headers into every response.
//! Protects against XSS, MIME-sniffing, clickjacking, and other common attacks.
//!
//! # Example
//! ```ignore
//! use floz::prelude::*;
//!
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(SecurityHeaders::default())
//!             // or with HSTS for production:
//!             .with_middleware(SecurityHeaders::strict())
//!     )
//!     .run()
//!     .await
//! ```

use crate::middleware::Middleware;
use ntex::http::header::HeaderValue;
use ntex::web::{HttpRequest, HttpResponse};

/// Security response headers middleware.
///
/// Injects the following headers into every response:
///
/// | Header | Effect |
/// |--------|--------|
/// | `X-Content-Type-Options: nosniff` | Prevents MIME-type sniffing |
/// | `X-Frame-Options: DENY` | Blocks iframe embedding (clickjacking) |
/// | `X-XSS-Protection: 0` | Disables legacy browser XSS auditor (modern CSP is better) |
/// | `Referrer-Policy: strict-origin-when-cross-origin` | Limits Referer leakage |
/// | `Content-Security-Policy: default-src 'self'` | Baseline CSP (customizable) |
/// | `Strict-Transport-Security` | (optional) Forces HTTPS (enable via `strict()`) |
///
/// # Customization
///
/// ```ignore
/// SecurityHeaders::default()
///     .with_hsts(31536000)                          // 1 year HSTS
///     .with_csp("default-src 'self'; img-src *")    // custom CSP
///     .with_frame_options("SAMEORIGIN")             // allow same-origin framing
/// ```
#[derive(Clone, Debug)]
pub struct SecurityHeaders {
    /// Content-Security-Policy header value
    csp: String,
    /// X-Frame-Options value
    frame_options: String,
    /// Referrer-Policy value
    referrer_policy: String,
    /// HSTS max-age in seconds (0 = disabled)
    hsts_max_age: u64,
    /// Include subdomains in HSTS
    hsts_include_subdomains: bool,
}

impl SecurityHeaders {
    /// "Relaxed" defaults — suitable for development and API servers.
    ///
    /// No HSTS (doesn't force HTTPS), permissive CSP for API responses.
    pub fn new() -> Self {
        Self {
            csp: "default-src 'none'; frame-ancestors 'none'".to_string(),
            frame_options: "DENY".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            hsts_max_age: 0,
            hsts_include_subdomains: false,
        }
    }

    /// Production-hardened defaults — enables HSTS with a 1-year max-age.
    ///
    /// Use this when your server is behind TLS termination and you want
    /// to enforce HTTPS for all clients.
    pub fn strict() -> Self {
        Self {
            hsts_max_age: 31_536_000, // 1 year
            hsts_include_subdomains: true,
            ..Self::new()
        }
    }

    /// Set a custom Content-Security-Policy header.
    pub fn with_csp(mut self, csp: &str) -> Self {
        self.csp = csp.to_string();
        self
    }

    /// Set the X-Frame-Options value (e.g. "DENY", "SAMEORIGIN").
    pub fn with_frame_options(mut self, value: &str) -> Self {
        self.frame_options = value.to_string();
        self
    }

    /// Set the Referrer-Policy value.
    pub fn with_referrer_policy(mut self, value: &str) -> Self {
        self.referrer_policy = value.to_string();
        self
    }

    /// Enable HSTS with the given max-age in seconds.
    /// Typical values: 31_536_000 (1 year), 63_072_000 (2 years).
    pub fn with_hsts(mut self, max_age: u64) -> Self {
        self.hsts_max_age = max_age;
        self
    }

    /// Include subdomains in HSTS directive.
    pub fn with_hsts_subdomains(mut self) -> Self {
        self.hsts_include_subdomains = true;
        self
    }
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for SecurityHeaders {
    fn handle(&self, _req: &HttpRequest) -> Option<HttpResponse> {
        None // always continue
    }

    fn response(&self, _req: &HttpRequest, mut resp: HttpResponse) -> HttpResponse {
        let headers = resp.headers_mut();

        // Prevent MIME-type sniffing
        headers.insert(
            ntex::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );

        // Prevent clickjacking
        if let Ok(value) = HeaderValue::from_str(&self.frame_options) {
            headers.insert(
                ntex::http::header::X_FRAME_OPTIONS,
                value,
            );
        }

        // Disable legacy XSS auditor (modern CSP is the replacement)
        headers.insert(
            ntex::http::header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("0"),
        );

        // Content-Security-Policy
        if !self.csp.is_empty() {
            if let Ok(value) = HeaderValue::from_str(&self.csp) {
                headers.insert(
                    ntex::http::header::HeaderName::from_static("content-security-policy"),
                    value,
                );
            }
        }

        // Referrer-Policy
        if let Ok(value) = HeaderValue::from_str(&self.referrer_policy) {
            headers.insert(
                ntex::http::header::HeaderName::from_static("referrer-policy"),
                value,
            );
        }

        // Permissions-Policy — restrict powerful browser features
        headers.insert(
            ntex::http::header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        );

        // HSTS (only if max-age > 0)
        if self.hsts_max_age > 0 {
            let hsts_value = if self.hsts_include_subdomains {
                format!("max-age={}; includeSubDomains", self.hsts_max_age)
            } else {
                format!("max-age={}", self.hsts_max_age)
            };
            if let Ok(value) = HeaderValue::from_str(&hsts_value) {
                headers.insert(
                    ntex::http::header::STRICT_TRANSPORT_SECURITY,
                    value,
                );
            }
        }

        resp
    }

    fn name(&self) -> &str {
        "security-headers"
    }
}
