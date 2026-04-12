//! Declarative Rate Limiting middleware.
//!
//! Implements `AsyncMiddleware` to intercept requests and enforce
//! access limits based on the client IP address (and optional Session ID).
//! Rate limits are automatically loaded from `RouteEntry.rate` or the global fallback.
//!
//! # Example
//! ```ignore
//! // In your route handler
//! #[route(get: "/tokens", rate = "5/sec")]
//! async fn get_token(...) -> HttpResponse { ... }
//! ```

use crate::middleware::pipeline::AsyncMiddleware;
use crate::router::RateLimitRouteMap;
use ntex::web::{HttpRequest, HttpResponse};

/// Rate limit middleware.
///
/// Looks up limits from the `RateLimitRouteMap` injected at app boot.
/// Connects to the Redis broker via `AppContext` if the `worker` feature is enabled.
/// Safely skips enforcement if Redis is disconnected.
#[derive(Clone)]
pub struct RateLimitMiddleware;

impl RateLimitMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Extrapolate a numeric limit and window (in seconds) from strings like "100/min".
    fn parse_rate(rate_str: &str) -> Option<(i64, u64)> {
        let parts: Vec<&str> = rate_str.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let limit: i64 = parts[0].parse().ok()?;
        let window = match parts[1].to_lowercase().as_str() {
            "sec" | "second" | "s" => 1,
            "min" | "minute" | "m" => 60,
            "hour" | "h" => 3600,
            "day" | "d" => 86400,
            _ => return None,
        };

        Some((limit, window))
    }
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncMiddleware for RateLimitMiddleware {
    async fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        // 1. Get the shared route map from app state
        let map = req.app_state::<RateLimitRouteMap>()?;

        // 2. Identify the matching route key
        let method = req.method().as_str();
        let path = req.path();
        
        let mut rate_str = None;
        
        // Fast path: try exact match first
        let exact_key = format!("{} {}", method, path);
        if let Some(rate) = map.get(&exact_key) {
            rate_str = Some(rate.clone());
        } else {
            // For parameterized routes, construct matching pattern
            let match_info = req.match_info();
            if !match_info.is_empty() {
                let mut pattern = path.to_string();
                for (key, value) in match_info.iter() {
                    pattern = pattern.replace(value, &format!("{{{}}}", key));
                }
                let pattern_key = format!("{} {}", method, pattern);
                if let Some(rate) = map.get(&pattern_key) {
                    rate_str = Some(rate.clone());
                }
            }
        }

        let Some(rate_str) = rate_str else {
            return None; // No rate limit configured for this route
        };

        let Some((limit, window_secs)) = Self::parse_rate(&rate_str) else {
            tracing::warn!("Invalid rate limit format: {}", rate_str);
            return None;
        };

        // 3. Obtain client identity (IP Address)
        let ip = req.connection_info().remote().unwrap_or("unknown").to_string();
        
        // Try locating session ID if configured in the pipeline extension
        let session_id = req.extensions().get::<crate::app::RequestContext>().map(|c| c.session_id.clone());
        
        let identity = if let Some(sid) = session_id {
            format!("{}:{}", ip, sid)
        } else {
            ip
        };

        #[cfg(feature = "worker")]
        {
            // 4. Redis sliding window enforcement
            let ctx = req.app_state::<crate::app::AppContext>()?;
            let Some(cache) = ctx.cache.as_ref() else {
                return None; // Silently skip if Redis connection was unconfigured
            };

            let block_key = format!("floz:rate:{}:{}:{}", identity, method, path);
            let mut conn = cache.connection();
            
            // Increment the specific endpoint counter
            use redis::AsyncCommands;
            let current_count: redis::RedisResult<i64> = conn.incr(&block_key, 1).await;
            
            match current_count {
                Ok(count) => {
                    if count == 1 {
                        // First hit in window, set the TTL
                        let _: () = conn.expire(&block_key, window_secs as i64).await.unwrap_or(());
                    }

                    if count > limit {
                        tracing::debug!("Rate limit exceeded for {} on {}", identity, path);
                        return Some(
                            HttpResponse::TooManyRequests()
                                .content_type("application/json")
                                .set_header("Retry-After", window_secs.to_string())
                                .body(format!(r#"{{"error": "Too Many Requests", "limit": "{}"}}"#, rate_str))
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Rate limiter Redis failure: {}", e);
                }
            }
        }
        
        #[cfg(not(feature = "worker"))]
        {
            // The `worker` feature is disabled. We bypass the check.
            tracing::trace!("Rate limit bypass: floz 'worker' feature is disabled (No Redis)");
        }

        None
    }

    async fn response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        resp
    }
}
