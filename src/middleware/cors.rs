//! CORS middleware for floz.
//!
//! # Example
//! ```ignore
//! use floz::middleware::cors::Cors;
//!
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(Cors::permissive())
//!             // or fine-grained:
//!             .with_middleware(
//!                 Cors::new()
//!                     .allow_origin("https://example.com")
//!                     .allow_methods(["GET", "POST"])
//!                     .supports_credentials()
//!             )
//!     )
//!     .run()
//!     .await
//! ```

use crate::middleware::Middleware;
use http::Method;
use ntex::http::header::{self, HeaderName, HeaderValue};
use ntex::web::{HttpRequest, HttpResponse};
use std::collections::HashSet;

/// CORS middleware for floz.
///
/// - `handle()`: intercepts preflight OPTIONS → early exit
/// - `response()`: applies CORS headers to all responses
#[derive(Clone)]
pub struct Cors {
    allowed_origins: HashSet<String>,
    allowed_methods: HashSet<String>,
    allowed_headers: HashSet<HeaderName>,
    exposed_headers: HashSet<HeaderName>,
    allow_credentials: bool,
    max_age: Option<usize>,
}

impl Cors {
    pub fn new() -> Self {
        Self::default()
    }

    /// Permissive CORS policy — allows all origins.
    /// Suitable for development; restrict in production.
    pub fn permissive() -> Self {
        Self::default()
            .allow_methods(
                ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"]
                    .iter()
                    .map(|&s| s.to_string()),
            )
            .allow_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600)
    }

    pub fn allow_origin(mut self, origin: &str) -> Self {
        self.allowed_origins.insert(origin.to_string());
        self
    }

    pub fn allow_origins<I>(mut self, origins: I) -> Self
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        self.allowed_origins = origins
            .into_iter()
            .map(|m| m.to_string().to_uppercase())
            .collect();
        self
    }

    pub fn allow_methods<I>(mut self, methods: I) -> Self
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        self.allowed_methods = methods
            .into_iter()
            .map(|m| m.to_string().to_uppercase())
            .collect();
        self
    }

    pub fn allow_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<HeaderName>,
    {
        self.allowed_headers = headers.into_iter().map(|h| h.into()).collect();
        self
    }

    pub fn expose_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<HeaderName>,
    {
        self.exposed_headers = headers.into_iter().map(|h| h.into()).collect();
        self
    }

    pub fn supports_credentials(mut self) -> Self {
        self.allow_credentials = true;
        self
    }

    pub fn max_age(mut self, max_age: usize) -> Self {
        self.max_age = Some(max_age);
        self
    }

    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.is_empty() || self.allowed_origins.contains(origin)
    }

    fn build_preflight_response(&self, req: &HttpRequest) -> HttpResponse {
        let mut builder = HttpResponse::Ok();

        if let Some(origin) = req.headers().get(header::ORIGIN) {
            if let Ok(origin_str) = origin.to_str() {
                if self.is_origin_allowed(origin_str) {
                    builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_str);
                    if self.allow_credentials {
                        builder.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
                    }
                } else {
                    return HttpResponse::Forbidden().finish();
                }
            }
        }

        if let Some(requested_method) = req.headers().get(header::ACCESS_CONTROL_REQUEST_METHOD) {
            if let Ok(method) = requested_method.to_str() {
                if self.allowed_methods.contains(&method.to_uppercase()) {
                    builder.header(
                        header::ACCESS_CONTROL_ALLOW_METHODS,
                        self.allowed_methods
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                }
            }
        }

        if let Some(requested_headers) = req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
            if let Ok(headers) = requested_headers.to_str() {
                let valid_headers: Vec<_> = headers
                    .split(',')
                    .map(|h| h.trim())
                    .filter(|&h| {
                        HeaderName::try_from(h)
                            .map(|name| self.allowed_headers.contains(&name))
                            .unwrap_or(false)
                    })
                    .collect();

                if !valid_headers.is_empty() {
                    builder.header(
                        header::ACCESS_CONTROL_ALLOW_HEADERS,
                        valid_headers.join(", "),
                    );
                }
            }
        }

        if let Some(max_age) = self.max_age {
            builder.header(header::ACCESS_CONTROL_MAX_AGE, max_age.to_string());
        }

        builder.finish()
    }

    fn apply_cors_headers(&self, response: &mut HttpResponse, origin: Option<String>) {
        if let Some(origin) = origin {
            if self.is_origin_allowed(&origin) {
                if let Ok(value) = HeaderValue::from_str(&origin) {
                    response
                        .headers_mut()
                        .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
                }

                if self.allow_credentials {
                    response.headers_mut().insert(
                        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        HeaderValue::from_static("true"),
                    );
                }

                if !self.exposed_headers.is_empty() {
                    let exposed = self
                        .exposed_headers
                        .iter()
                        .map(|h| h.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if let Ok(value) = HeaderValue::from_str(&exposed) {
                        response
                            .headers_mut()
                            .insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, value);
                    }
                }
            }
        }
    }
}

impl Default for Cors {
    fn default() -> Self {
        Cors {
            allowed_origins: HashSet::new(),
            allowed_methods: HashSet::from([
                "GET".to_string(),
                "POST".to_string(),
                "OPTIONS".to_string(),
            ]),
            allowed_headers: HashSet::from([
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ]),
            exposed_headers: HashSet::new(),
            allow_credentials: false,
            max_age: Some(3600),
        }
    }
}

impl Middleware for Cors {
    fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        if req.method() == Method::OPTIONS {
            Some(self.build_preflight_response(req))
        } else {
            None
        }
    }

    fn response(&self, req: &HttpRequest, mut resp: HttpResponse) -> HttpResponse {
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string());

        self.apply_cors_headers(&mut resp, origin);
        resp
    }

    fn name(&self) -> &str {
        "cors"
    }
}
