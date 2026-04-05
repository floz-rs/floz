//! Middleware pipeline example.
//!
//! Demonstrates the full floz middleware stack:
//! - CORS (preflight handling)
//! - RequestTrace (structured logging)
//! - Compression (gzip response bodies)
//! - Custom auth middleware (early exit / halt)
//!
//! Run:
//!   cargo run --bin middleware_pipeline
//!
//! Test:
//!   # Public endpoint (no auth needed)
//!   curl http://localhost:8080/health
//!
//!   # Protected endpoint (rejected)
//!   curl http://localhost:8080/secret
//!
//!   # Protected endpoint (accepted)
//!   curl -H "Authorization: Bearer my-secret-token" http://localhost:8080/secret
//!
//!   # Preflight CORS
//!   curl -X OPTIONS -H "Origin: https://example.com" http://localhost:8080/health

use floz::prelude::*;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Custom Auth Middleware
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Simple bearer token auth middleware.
///
/// Returns 401 Unauthorized if the token is missing or wrong.
/// This demonstrates the "early exit" pattern — `handle()` returns
/// `Some(HttpResponse)` to halt the pipeline before the handler runs.
#[derive(Clone)]
struct BearerAuth {
    token: String,
}

impl BearerAuth {
    fn new(token: &str) -> Self {
        Self {
            token: format!("Bearer {}", token),
        }
    }
}

impl Middleware for BearerAuth {
    fn handle(&self, req: &Req) -> Option<Resp> {
        // Skip auth for public paths
        if req.path() == "/health" {
            return None; // continue — no auth needed
        }

        // Check Authorization header
        let auth = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok());

        match auth {
            Some(t) if t == self.token => None, // valid — continue
            _ => Some(
                Resp::Unauthorized()
                    .json(&json!({
                        "error": "unauthorized",
                        "message": "Invalid or missing bearer token"
                    })),
            ),
        }
    }

    fn name(&self) -> &str {
        "bearer_auth"
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Custom Timing Middleware
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Adds an `X-Response-Time` header to every response.
///
/// This demonstrates the `response()` post-processing pattern —
/// the middleware modifies the response after the handler runs.
#[derive(Clone)]
struct ResponseTimer;

impl Middleware for ResponseTimer {
    fn handle(&self, _req: &Req) -> Option<Resp> {
        None // always continue
    }

    fn response(&self, _req: &Req, mut resp: Resp) -> Resp {
        // In a real implementation, you'd capture the start time
        // in handle() and compute elapsed here. Since our middleware
        // is sync and can't store per-request state easily, we just
        // demonstrate the header injection pattern.
        resp.headers_mut().insert(
            ntex::http::header::HeaderName::from_static("x-powered-by"),
            ntex::http::header::HeaderValue::from_static("floz"),
        );
        resp
    }

    fn name(&self) -> &str {
        "response_timer"
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Route Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /health — public, no auth needed
#[route(get: "/health", tag: "System", desc: "Health check")]
async fn health() -> Resp {
    Resp::Ok().json(&json!({
        "status": "ok",
        "middleware": ["cors", "trace", "compression", "bearer_auth", "response_timer"]
    }))
}

/// GET /secret — protected by BearerAuth
#[route(get: "/secret", tag: "Protected", desc: "Secret endpoint")]
async fn secret() -> Resp {
    Resp::Ok().json(&json!({
        "message": "You have access!",
        "data": "This is the secret payload."
    }))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Main
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[floz::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("DATABASE_URL").is_err() {
        std::env::set_var("DATABASE_URL", "postgres://localhost:5432/floz1");
    }

    App::new()
        .server(
            ServerConfig::new()
                .with_default_port(8080)
                // Middleware runs in this order:
                // 1. CORS — handle preflight OPTIONS early
                .with_middleware(Cors::permissive())
                // 2. Tracing — log every request/response
                .with_middleware(RequestTrace::default())
                // 3. Compression — gzip large responses
                .with_middleware(Compression::gzip())
                // 4. Auth — reject unauthorized requests
                .with_middleware(BearerAuth::new("my-secret-token"))
                // 5. Timer — add X-Powered-By header
                .with_middleware(ResponseTimer),
        )
        .run()
        .await
}
