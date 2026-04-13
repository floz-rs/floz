use crate::app::RequestContext;
use crate::middleware::Middleware;
use ntex::http::header::{HeaderValue, COOKIE, SET_COOKIE};
use ntex::web::{HttpRequest, HttpResponse};
use uuid::Uuid;

/// Session Management Middleware.
///
/// Ensures every request has a tracking session. Parses the `floz_session`
/// cookie. If missing, it generates a new UUIDv4 and automatically injects
/// a `Set-Cookie` header into the outgoing response.
///
/// It establishes the `RequestContext` which is injected into `floz::web::Context`.
#[derive(Clone, Debug, Default)]
pub struct SessionMiddleware;

impl SessionMiddleware {
    pub fn new() -> Self {
        Self
    }
}

const COOKIE_NAME: &str = "floz_session";

struct NewSessionMarker(String);

impl Middleware for SessionMiddleware {
    fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        let mut session_id = None;

        // Try to parse cookie manually
        for val in req.headers().get_all(COOKIE) {
            if let Ok(s) = val.to_str() {
                for pair in s.split(';') {
                    let pair = pair.trim();
                    if let Some(cookie_val) = pair.strip_prefix(&format!("{}=", COOKIE_NAME)) {
                        session_id = Some(cookie_val.to_string());
                        break;
                    }
                }
            }
        }

        let is_new = session_id.is_none();
        let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        // Insert the primary RequestContext
        req.extensions_mut().insert(RequestContext {
            session_id: session_id.clone(),
            auth: Default::default(),
        });

        // Store a marker if it's a new session, so we can set the cookie in `response`
        if is_new {
            req.extensions_mut().insert(NewSessionMarker(session_id));
        }

        None
    }

    fn response(&self, req: &HttpRequest, mut resp: HttpResponse) -> HttpResponse {
        // If we generated a new session ID for this request, set the cookie header
        if let Some(marker) = req.extensions_mut().remove::<NewSessionMarker>() {
            // Add `Secure` flag in production to prevent cookie transmission over HTTP
            let is_prod = std::env::var("SERVER_ENV")
                .map(|v| v.eq_ignore_ascii_case("PROD"))
                .unwrap_or(false);

            let cookie_str = if is_prod {
                format!(
                    "{}={}; Path=/; HttpOnly; SameSite=Lax; Secure",
                    COOKIE_NAME, marker.0
                )
            } else {
                format!(
                    "{}={}; Path=/; HttpOnly; SameSite=Lax",
                    COOKIE_NAME, marker.0
                )
            };
            if let Ok(hv) = HeaderValue::from_str(&cookie_str) {
                resp.headers_mut().append(SET_COOKIE, hv);
            }
        }
        resp
    }

    fn name(&self) -> &str {
        "session"
    }
}
