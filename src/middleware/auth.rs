//! Global Authorization Middleware.
//!
//! Enforces SecurityRouteRules extracted from `#[route(auth, permissions=[...])]`.

use crate::app::AppContext;
use crate::app::{AuthInfo, RequestContext};
use crate::middleware::pipeline::AsyncMiddleware;
use crate::router::{RouteSecurityRule, SecurityRouteMap};
use ntex::web::{HttpRequest, HttpResponse};

/// Resolves user identity automatically (JWT or Session) and enforces route security rules dynamically.
#[derive(Clone, Default)]
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Attempts to locate a matching `RouteSecurityRule` by checking exact match or `match_info()` patterns.
    fn find_security_rule<'a>(
        req: &HttpRequest,
        map: &'a std::collections::HashMap<String, RouteSecurityRule>,
    ) -> Option<&'a RouteSecurityRule> {
        let method = req.method().as_str();
        let path = req.path();

        let exact_key = format!("{} {}", method, path);
        if let Some(rule) = map.get(&exact_key) {
            return Some(rule);
        }

        let match_info = req.match_info();
        if !match_info.is_empty() {
            let mut pattern = path.to_string();
            // Important limitation: Ntex clears `path` if it isn't raw. But standard mapping is {param}.
            for (key, value) in match_info.iter() {
                pattern = pattern.replace(value, &format!("{{{}}}", key));
            }
            let pattern_key = format!("{} {}", method, pattern);
            if let Some(rule) = map.get(&pattern_key) {
                return Some(rule);
            }
        }
        None
    }
}

impl AsyncMiddleware for AuthMiddleware {
    async fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        // 1. Check if route even has security constraints
        let security_map = req.app_state::<SecurityRouteMap>();
        let rule = security_map
            .as_ref()
            .and_then(|map| Self::find_security_rule(req, map));

        let app = req.app_state::<AppContext>();
        let mut extensions = req.extensions_mut();

        let mut req_ctx = extensions
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_else(|| RequestContext {
                session_id: "anonymous_stub".to_string(),
                auth: AuthInfo::default(),
            });

        // 2. Extract Identity (Try JWT Bearer Token first)
        let mut parsed_identity = false;
        if let Some(app_ctx) = app.as_ref() {
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];

                        // SECURITY: Refuse to use a default secret in production.
                        let secret = match app_ctx.config.jwt_secret.as_deref() {
                            Some(s) => s,
                            None => {
                                let is_prod =
                                    app_ctx.config.server_env.eq_ignore_ascii_case("PROD");
                                if is_prod {
                                    tracing::error!("JWT_TOKEN env var is not set! Rejecting all Bearer tokens in production.");
                                    return Some(HttpResponse::InternalServerError().json(
                                        &serde_json::json!({
                                            "error": "server_misconfigured",
                                            "message": "Authentication is not configured."
                                        }),
                                    ));
                                }
                                tracing::warn!("⚠️  JWT_TOKEN env var is not set — using insecure default. DO NOT use in production!");
                                "floz-dev-secret-do-not-use-in-production"
                            }
                        };

                        let audience = app_ctx.config.jwt_audience.as_deref().unwrap_or("floz-api");
                        let issuer = app_ctx.config.jwt_issuer.as_deref().unwrap_or("floz");

                        if let Ok(claims) = crate::auth::jwt::verify_token(
                            token,
                            secret.as_bytes(),
                            audience,
                            issuer,
                        ) {
                            req_ctx.auth = AuthInfo {
                                user_id: Some(claims.sub),
                                roles: vec![claims.role],
                                permissions: vec![], // Typically JWT doesn't embed all permissions
                            };
                            parsed_identity = true;
                        }
                    }
                }
            }
        }

        // 3. Fallback to Redis Session (if JWT didn't authenticate them)
        if !parsed_identity {
            #[cfg(feature = "worker")]
            if let Some(app_ctx) = app.as_ref() {
                let session_store = req_ctx.session(app_ctx);
                if let Ok(Some(auth_info)) = session_store.get::<AuthInfo>("_floz_auth_info").await
                {
                    req_ctx.auth = auth_info;
                    parsed_identity = true;
                }
            }
        }

        // 4. Update the RequestContext in the extension map so downstream handlers can access it!
        extensions.insert(req_ctx.clone());
        drop(extensions);

        // 5. Evaluate Rules dynamically
        if let Some(rule) = rule {
            // Require ANY form of auth (if auth != none)
            if rule.auth.is_some() || !rule.permissions.is_empty() {
                if !parsed_identity {
                    return Some(HttpResponse::Unauthorized().json(&serde_json::json!({
                        "error": "unauthorized",
                        "code": "missing_identity",
                        "message": "Valid Authentication token or session was not found."
                    })));
                }

                // User is authenticated, now verify all permissions
                for perm in &rule.permissions {
                    if !req_ctx.auth.has_permission(perm) {
                        return Some(HttpResponse::Forbidden().json(&serde_json::json!({
                            "error": "forbidden",
                            "code": "missing_permission",
                            "message": format!("Missing required permission: {}", perm)
                        })));
                    }
                }
            }
        }

        None
    }

    async fn response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        resp
    }
}
