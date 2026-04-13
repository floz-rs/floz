//! Declarative HTTP response caching middleware.
//!
//! Implements `AsyncMiddleware` to intercept requests and serve
//! cached responses from Redis when available, and cache new
//! responses on the way out.
//!
//! # How it works
//!
//! 1. On `handle()`: builds a cache key from `METHOD path?query`,
//!    checks Redis — if found, returns the cached response immediately.
//! 2. On `response()`: if the route has `cache_ttl` configured,
//!    serializes the response body to Redis and tags it for
//!    automatic invalidation by the Outbox Sweeper.
//!
//! # Example
//! ```ignore
//! // In your route handler — zero boilerplate needed!
//! #[route(get: "/users/:id", cache(ttl = 600, watch = ["users"]))]
//! async fn get_user(...) -> HttpResponse { ... }
//! ```

use crate::middleware::pipeline::AsyncMiddleware;
use crate::router::CacheRouteInfo;
use ntex::web::{HttpRequest, HttpResponse};
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for the shared cache route map injected into ntex app state.
pub type CacheRouteMap = Arc<HashMap<String, CacheRouteInfo>>;

/// Declarative cache middleware — uses `AsyncMiddleware` for
/// non-blocking Redis cache lookups and writes.
///
/// Added to the pipeline automatically or manually:
/// ```ignore
/// App::new()
///     .server(
///         ServerConfig::new()
///             .with_async_middleware(CacheMiddleware)
///     )
///     .run()
///     .await
/// ```
#[derive(Clone, Default)]
pub struct CacheMiddleware;

impl CacheMiddleware {
    /// Build the cache key for a request: `"GET /users/42?sort=name"`
    fn cache_key(req: &HttpRequest) -> String {
        let method = req.method().as_str();
        let path = req.path();
        let qs = req.query_string();
        if qs.is_empty() {
            format!("floz:cache:resp:{}:{}", method, path)
        } else {
            format!("floz:cache:resp:{}:{}?{}", method, path, qs)
        }
    }

    /// Resolve actual tag values by substituting path params.
    ///
    /// For example, `"users:{id}"` with path param `id=42` → `"users:42"`
    fn resolve_tags(tags: &[&str], req: &HttpRequest) -> Vec<String> {
        let match_info = req.match_info();
        tags.iter()
            .map(|tag| {
                let mut resolved = tag.to_string();
                // Replace `{param}` patterns in tags with actual values
                for (key, value) in match_info.iter() {
                    let placeholder = format!("{{{}}}", key);
                    resolved = resolved.replace(&placeholder, value);
                    // Also support `:param` style in tags
                    let colon_placeholder = format!(":{}", key);
                    resolved = resolved.replace(&colon_placeholder, value);
                }
                resolved
            })
            .collect()
    }

    /// Find the matching CacheRouteInfo for this request by checking
    /// the route map patterns against the incoming path.
    fn find_cache_info<'a>(
        req: &HttpRequest,
        map: &'a HashMap<String, CacheRouteInfo>,
    ) -> Option<&'a CacheRouteInfo> {
        let method = req.method().as_str();
        let path = req.path();

        // Fast path: try exact match first (non-parameterized routes)
        let exact_key = format!("{} {}", method, path);
        if let Some(info) = map.get(&exact_key) {
            return Some(info);
        }

        // For parameterized routes, we need pattern matching.
        // Since ntex has already matched and extracted params, we can
        // reconstruct the pattern from path + match_info.
        // Build the pattern by replacing param values back with {param} placeholders.
        let match_info = req.match_info();
        if !match_info.is_empty() {
            let mut pattern = path.to_string();
            for (key, value) in match_info.iter() {
                pattern = pattern.replace(value, &format!("{{{}}}", key));
            }
            let pattern_key = format!("{} {}", method, pattern);
            if let Some(info) = map.get(&pattern_key) {
                return Some(info);
            }
        }

        None
    }
}

impl AsyncMiddleware for CacheMiddleware {
    async fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        // 1. Get the shared route map from app state
        let map = req.app_state::<CacheRouteMap>()?;

        // 2. Check if this route has caching configured
        let _cache_info = Self::find_cache_info(req, map)?;

        // 3. Get the Redis cache from AppContext
        #[cfg(feature = "worker")]
        {
            let ctx = req.app_state::<crate::app::AppContext>()?;
            let cache = ctx.cache.as_ref()?;

            // 4. Build cache key and check Redis
            let key = Self::cache_key(req);
            match cache.get(&key).await {
                Ok(Some(cached_body)) => {
                    tracing::debug!("Cache HIT: {}", key);
                    // Return the cached response directly — skip the handler entirely
                    return Some(
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body(cached_body),
                    );
                }
                Ok(None) => {
                    tracing::debug!("Cache MISS: {}", key);
                }
                Err(e) => {
                    tracing::warn!("Cache lookup error: {}", e);
                }
            }
        }

        // Continue to handler
        None
    }

    async fn response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        // Only cache successful JSON responses
        if !resp.status().is_success() {
            return resp;
        }

        // 1. Get the shared route map from app state
        let Some(map) = req.app_state::<CacheRouteMap>() else {
            return resp;
        };

        // 2. Check if this route has caching configured
        let Some(cache_info) = Self::find_cache_info(req, map) else {
            return resp;
        };

        #[cfg(feature = "worker")]
        {
            let ctx = req.app_state::<crate::app::AppContext>();
            if let Some(ctx) = ctx {
                if let Some(ref cache) = ctx.cache {
                    // 3. Extract the response body as bytes
                    let body = resp.body();
                    let body_bytes: Option<&[u8]> = match body {
                        ntex::http::body::ResponseBody::Body(ntex::http::body::Body::Bytes(
                            ref b,
                        )) => Some(b.as_ref()),
                        ntex::http::body::ResponseBody::Other(ntex::http::body::Body::Bytes(
                            ref b,
                        )) => Some(b.as_ref()),
                        _ => None,
                    };
                    if let Some(raw) = body_bytes {
                        if let Ok(body_str) = std::str::from_utf8(raw) {
                            let key = Self::cache_key(req);

                            // 4. Write response to Redis with TTL
                            if let Err(e) = cache.set(&key, body_str, cache_info.ttl).await {
                                tracing::warn!("Cache write error: {}", e);
                            } else {
                                tracing::debug!("Cache SET: {} (ttl={}s)", key, cache_info.ttl);

                                // 5. Tag the cache key for invalidation
                                let resolved_tags = Self::resolve_tags(&cache_info.watch, req);
                                for tag in &resolved_tags {
                                    let _ = cache.add_tag(tag, &key).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        resp
    }
}
