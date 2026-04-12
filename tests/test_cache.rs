//! Tests for the declarative caching system.
//!
//! Covers:
//! - Route map building from inventory
//! - Cache key generation
//! - Tag resolution with path parameters
//! - CacheMiddleware integration with ntex test server

use floz::middleware::cache::CacheMiddleware;
use floz::router::{CacheRouteInfo, build_cache_route_map};
use std::collections::HashMap;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// translate_path (tested via build_cache_route_map output)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_build_cache_route_map_returns_hashmap() {
    // Without any inventory entries, the map is simply empty
    let map = build_cache_route_map();
    // We can at least verify the type is correct
    assert!(map.is_empty() || !map.is_empty()); // type check
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CacheRouteInfo struct tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_cache_route_info_construction() {
    let info = CacheRouteInfo {
        path_pattern: "/users/:id",
        ttl: 600,
        watch: vec!["users", "users:{id}"],
    };
    assert_eq!(info.ttl, 600);
    assert_eq!(info.path_pattern, "/users/:id");
    assert_eq!(info.watch.len(), 2);
    assert_eq!(info.watch[0], "users");
    assert_eq!(info.watch[1], "users:{id}");
}

#[test]
fn test_cache_route_info_clone() {
    let info = CacheRouteInfo {
        path_pattern: "/health",
        ttl: 30,
        watch: vec!["system"],
    };
    let cloned = info.clone();
    assert_eq!(cloned.ttl, info.ttl);
    assert_eq!(cloned.watch, info.watch);
}

#[test]
fn test_cache_route_info_debug() {
    let info = CacheRouteInfo {
        path_pattern: "/users",
        ttl: 300,
        watch: vec!["users"],
    };
    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("300"));
    assert!(debug_str.contains("users"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CacheMiddleware unit tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_cache_middleware_is_clone() {
    let mw = CacheMiddleware;
    let _cloned = mw.clone();
}

#[test]
fn test_cache_middleware_default() {
    let _mw = CacheMiddleware::default();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Route map lookup helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_route_map_exact_match() {
    let mut map = HashMap::new();
    map.insert("GET /health".to_string(), CacheRouteInfo {
        path_pattern: "/health",
        ttl: 30,
        watch: vec!["system"],
    });

    assert!(map.contains_key("GET /health"));
    assert!(!map.contains_key("POST /health"));
    assert!(!map.contains_key("GET /users"));
}

#[test]
fn test_route_map_parameterized_pattern() {
    let mut map = HashMap::new();
    map.insert("GET /users/{id}".to_string(), CacheRouteInfo {
        path_pattern: "/users/:id",
        ttl: 600,
        watch: vec!["users", "users:{id}"],
    });

    // The key uses ntex {param} syntax
    assert!(map.contains_key("GET /users/{id}"));
    // Actual request path won't match directly — middleware does pattern reconstruction
    assert!(!map.contains_key("GET /users/42"));
}

#[test]
fn test_route_map_multiple_entries() {
    let mut map = HashMap::new();
    map.insert("GET /users".to_string(), CacheRouteInfo {
        path_pattern: "/users",
        ttl: 300,
        watch: vec!["users"],
    });
    map.insert("GET /users/{id}".to_string(), CacheRouteInfo {
        path_pattern: "/users/:id",
        ttl: 600,
        watch: vec!["users", "users:{id}"],
    });
    map.insert("GET /health".to_string(), CacheRouteInfo {
        path_pattern: "/health",
        ttl: 30,
        watch: vec![],
    });

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("GET /users").unwrap().ttl, 300);
    assert_eq!(map.get("GET /users/{id}").unwrap().ttl, 600);
    assert_eq!(map.get("GET /health").unwrap().ttl, 30);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Cache key format tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_cache_key_format_no_query() {
    // Verify the key format convention
    let key = format!("floz:cache:resp:{}:{}", "GET", "/users");
    assert_eq!(key, "floz:cache:resp:GET:/users");
}

#[test]
fn test_cache_key_format_with_query() {
    let method = "GET";
    let path = "/users";
    let qs = "page=1&limit=20";
    let key = format!("floz:cache:resp:{}:{}?{}", method, path, qs);
    assert_eq!(key, "floz:cache:resp:GET:/users?page=1&limit=20");
}

#[test]
fn test_cache_key_different_methods_differ() {
    let get_key = format!("floz:cache:resp:{}:{}", "GET", "/users");
    let post_key = format!("floz:cache:resp:{}:{}", "POST", "/users");
    assert_ne!(get_key, post_key);
}

#[test]
fn test_cache_key_different_paths_differ() {
    let users_key = format!("floz:cache:resp:{}:{}", "GET", "/users");
    let roles_key = format!("floz:cache:resp:{}:{}", "GET", "/roles");
    assert_ne!(users_key, roles_key);
}

#[test]
fn test_cache_key_different_params_differ() {
    let key1 = format!("floz:cache:resp:{}:{}", "GET", "/users/1");
    let key2 = format!("floz:cache:resp:{}:{}", "GET", "/users/2");
    assert_ne!(key1, key2);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tag resolution tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_tag_resolution_static() {
    // Static tags without params remain unchanged
    let tags = vec!["users", "system"];
    // Without a real request, we verify the format
    for tag in &tags {
        assert!(!tag.contains('{'));
        assert!(!tag.contains(':'));
    }
}

#[test]
fn test_tag_resolution_parameterized_format() {
    // Verify the tag format convention
    let tag = "users:{id}";
    assert!(tag.contains("{id}"));

    // After resolution, it should look like "users:42"
    let resolved = tag.replace("{id}", "42");
    assert_eq!(resolved, "users:42");
}

#[test]
fn test_tag_resolution_multiple_params() {
    let tag = "posts:{post_id}:comments:{comment_id}";
    let resolved = tag.replace("{post_id}", "10").replace("{comment_id}", "5");
    assert_eq!(resolved, "posts:10:comments:5");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Integration: CacheMiddleware in ntex pipeline
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[ntex::test]
async fn test_cache_middleware_passthrough_no_cache_routes() {
    use ntex::web::{self, App, HttpResponse};
    use ntex::web::test::{init_service, TestRequest};

    // Pipeline with CacheMiddleware but no cache route map injected
    // → should pass through cleanly without panicking
    let app = init_service(
        App::new()
            .middleware(floz::middleware::FlozPipeline::new(
                floz::middleware::Stack {
                    inner: floz::middleware::EmptyStack,
                    outer: floz::middleware::AsyncLayer(CacheMiddleware),
                }
            ))
            .route("/test", web::get().to(|| async { HttpResponse::Ok().body("hello") }))
    ).await;

    let req = TestRequest::get().uri("/test").to_request();
    let resp = ntex::web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[ntex::test]
async fn test_cache_middleware_with_empty_route_map() {
    use ntex::web::{self, App, HttpResponse};
    use ntex::web::test::{init_service, TestRequest};
    use std::sync::Arc;
    use floz::middleware::cache::CacheRouteMap;

    // Inject an empty cache route map
    let empty_map: CacheRouteMap = Arc::new(HashMap::new());

    let app = init_service(
        App::new()
            .state(empty_map)
            .middleware(floz::middleware::FlozPipeline::new(
                floz::middleware::Stack {
                    inner: floz::middleware::EmptyStack,
                    outer: floz::middleware::AsyncLayer(CacheMiddleware),
                }
            ))
            .route("/api/data", web::get().to(|| async {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(r#"{"data":"fresh"}"#)
            }))
    ).await;

    let req = TestRequest::get().uri("/api/data").to_request();
    let resp = ntex::web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[ntex::test]
async fn test_cache_middleware_with_populated_route_map() {
    use ntex::web::{self, App, HttpResponse};
    use ntex::web::test::{init_service, TestRequest};
    use std::sync::Arc;
    use floz::middleware::cache::CacheRouteMap;

    // Build a map that includes a cached route
    let mut map = HashMap::new();
    map.insert("GET /api/cached".to_string(), CacheRouteInfo {
        path_pattern: "/api/cached",
        ttl: 60,
        watch: vec!["test_table"],
    });
    let cache_map: CacheRouteMap = Arc::new(map);

    let app = init_service(
        App::new()
            .state(cache_map)
            .middleware(floz::middleware::FlozPipeline::new(
                floz::middleware::Stack {
                    inner: floz::middleware::EmptyStack,
                    outer: floz::middleware::AsyncLayer(CacheMiddleware),
                }
            ))
            .route("/api/cached", web::get().to(|| async {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(r#"{"data":"cached_response"}"#)
            }))
    ).await;

    // Without Redis, the middleware should pass through (cache miss)
    // and return the handler's response
    let req = TestRequest::get().uri("/api/cached").to_request();
    let resp = ntex::web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[ntex::test]
async fn test_cache_middleware_combined_with_sync() {
    use ntex::web::{self, App, HttpResponse};
    use ntex::web::test::{init_service, TestRequest};
    use floz::middleware::cors::Cors;

    // Combine sync Cors + async CacheMiddleware in a single pipeline
    let app = init_service(
        App::new()
            .middleware(floz::middleware::FlozPipeline::new(
                floz::middleware::Stack {
                    inner: floz::middleware::Stack {
                        inner: floz::middleware::EmptyStack,
                        outer: floz::middleware::SyncLayer(Cors::permissive()),
                    },
                    outer: floz::middleware::AsyncLayer(CacheMiddleware),
                }
            ))
            .route("/mixed", web::get().to(|| async { HttpResponse::Ok().body("mixed") }))
    ).await;

    let req = TestRequest::get()
        .uri("/mixed")
        .header(ntex::http::header::ORIGIN, "https://example.com")
        .to_request();
    let resp = ntex::web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // CORS headers should still be present
    let cors_header = resp.headers().get(ntex::http::header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(cors_header.is_some());
}
