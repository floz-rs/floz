use crate::middleware::Middleware;
use ntex::web::{HttpRequest, HttpResponse};
use ntex::http::{Method, StatusCode};
use ntex::http::header::{HeaderValue, SET_COOKIE, COOKIE};
use uuid::Uuid;

/// CSRF Protection Middleware.
///
/// Implements the Double Submit Cookie stateless CSRF pattern:
/// 1. Sets a `floz_csrf` cookie with a cryptographically secure token on every safe request if lacking.
/// 2. For mutating requests (POST, PUT, DELETE, PATCH), enforces that the `X-CSRF-Token` header matches the cookie.
/// 3. Safely bypasses enforcement if an `Authorization: Bearer` header is present (API clients).
#[derive(Clone, Debug)]
pub struct CsrfMiddleware {
    cookie_name: &'static str,
    header_name: &'static str,
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self {
            cookie_name: "floz_csrf",
            header_name: "x-csrf-token",
        }
    }
}

impl CsrfMiddleware {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct CsrfNewToken(pub String);

impl Middleware for CsrfMiddleware {
    fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        let is_mutating = match *req.method() {
            Method::POST | Method::PUT | Method::DELETE | Method::PATCH => true,
            _ => false,
        };

        // Bypass for APIs utilizing Authorization: Bearer
        if let Some(auth) = req.headers().get("Authorization") {
            if auth.to_str().unwrap_or("").starts_with("Bearer ") {
                return None;
            }
        }

        let mut cookie_token = None;
        for val in req.headers().get_all(COOKIE) {
            if let Ok(s) = val.to_str() {
                for pair in s.split(';') {
                    let pair = pair.trim();
                    if let Some(mutval) = pair.strip_prefix(&format!("{}=", self.cookie_name)) {
                        cookie_token = Some(mutval.to_string());
                        break;
                    }
                }
            }
        }

        let cookie_token = match cookie_token {
            Some(t) => t,
            None => {
                let token = Uuid::new_v4().to_string();
                req.extensions_mut().insert(CsrfNewToken(token.clone()));
                token
            }
        };

        if is_mutating {
            let header_token = req.headers().get(self.header_name).and_then(|h| h.to_str().ok());
            if header_token != Some(&cookie_token) {
                return Some(
                    HttpResponse::build(StatusCode::FORBIDDEN)
                        .body("CSRF validation failed: missing or invalid CSRF token")
                );
            }
        }

        None
    }

    fn response(&self, req: &HttpRequest, mut resp: HttpResponse) -> HttpResponse {
        if let Some(new_token) = req.extensions_mut().remove::<CsrfNewToken>() {
            let cookie_str = format!("{}={}; Path=/; HttpOnly; SameSite=Lax", self.cookie_name, new_token.0);
            if let Ok(val) = HeaderValue::from_str(&cookie_str) {
                resp.headers_mut().append(SET_COOKIE, val);
            }
        }
        resp
    }

    fn name(&self) -> &str {
        "csrf"
    }
}
