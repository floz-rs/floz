//! Testing utilities for floz applications.
//!
//! Provides `TestApp`, `TestRequest`, and `TestResponse` for writing
//! integration tests with minimal boilerplate.
//!
//! Uses ntex's built-in `TestServer` — zero additional dependencies.
//!
//! # Usage
//!
//! ```ignore
//! use floz::testing::TestApp;
//!
//! #[tokio::test]
//! async fn test_health_check() {
//!     let app = TestApp::new().await;
//!     let resp = app.get("/health").send().await;
//!     assert_eq!(resp.status(), 200);
//! }
//! ```

mod request;

use crate::app::AppContext;
use crate::middleware::pipeline::{EmptyStack, FlozPipeline};
use ntex::web;
use std::collections::HashMap;
use std::sync::Arc;

pub use request::{TestRequest, TestResponse};

/// A test application server with full route auto-discovery.
///
/// Wraps an ntex `TestServer` bound to a random port, with all
/// `#[route]` handlers registered and a real (or test) database connection.
///
/// # Example
///
/// ```ignore
/// let app = TestApp::new().await;
///
/// let resp = app.get("/users").send().await;
/// assert_eq!(resp.status(), 200);
///
/// let resp = app.post("/users")
///     .json(&json!({ "name": "Alice" }))
///     .send()
///     .await;
/// assert_eq!(resp.status(), 201);
/// ```
pub struct TestApp {
    /// The ntex test server
    pub(crate) server: ntex::web::test::TestServer,
    /// Default headers to send with every request
    pub(crate) default_headers: HashMap<String, String>,
}

impl TestApp {
    /// Create a new test app with default configuration.
    ///
    /// - Uses `TEST_DATABASE_URL` env var (falls back to `DATABASE_URL`)
    /// - Auto-discovers all `#[route]` handlers
    /// - Binds to a random available port
    pub async fn new() -> Self {
        Self::builder().build().await
    }

    /// Create a builder for more control over the test app configuration.
    pub fn builder() -> TestAppBuilder {
        TestAppBuilder::default()
    }

    /// Get the base URL of the running test server (e.g. "http://127.0.0.1:54321").
    pub fn url(&self, path: &str) -> String {
        self.server.url(path)
    }

    /// Create a GET request.
    pub fn get(&self, path: &str) -> TestRequest<'_> {
        TestRequest::new(self, "GET", path)
    }

    /// Create a POST request.
    pub fn post(&self, path: &str) -> TestRequest<'_> {
        TestRequest::new(self, "POST", path)
    }

    /// Create a PUT request.
    pub fn put(&self, path: &str) -> TestRequest<'_> {
        TestRequest::new(self, "PUT", path)
    }

    /// Create a PATCH request.
    pub fn patch(&self, path: &str) -> TestRequest<'_> {
        TestRequest::new(self, "PATCH", path)
    }

    /// Create a DELETE request.
    pub fn delete(&self, path: &str) -> TestRequest<'_> {
        TestRequest::new(self, "DELETE", path)
    }
}

/// Builder for configuring a `TestApp`.
pub struct TestAppBuilder {
    extensions: HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
    default_headers: HashMap<String, String>,
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self {
            extensions: HashMap::new(),
            default_headers: HashMap::new(),
        }
    }
}

impl TestAppBuilder {
    /// Register a custom state extension (available via `state.ext::<T>()`).
    pub fn with<T: Send + Sync + 'static>(mut self, data: T) -> Self {
        self.extensions
            .insert(std::any::TypeId::of::<T>(), Box::new(data));
        self
    }

    /// Add a default header sent with every request.
    pub fn default_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }

    /// Add a default Authorization Bearer token.
    pub fn bearer(self, token: &str) -> Self {
        self.default_header("Authorization", format!("Bearer {}", token))
    }

    /// Build and start the test server.
    pub async fn build(self) -> TestApp {
        // Ensure .env is loaded for test database
        dotenvy::dotenv().ok();

        // Set TEST_DATABASE_URL override if available
        if let Ok(test_url) = std::env::var("TEST_DATABASE_URL") {
            unsafe {
                std::env::set_var("DATABASE_URL", test_url);
            }
        }

        // Initialize AppContext with test extensions
        let ctx = AppContext::init(self.extensions).await;

        // Build cache route map
        let cache_route_map = Arc::new(crate::router::build_cache_route_map());

        // Start a test server on a random port using ntex's built-in TestServer.
        let srv = ntex::web::test::server(move || {
            let ctx = ctx.clone();
            let cache_map = cache_route_map.clone();
            let pipeline = FlozPipeline::new(EmptyStack);

            async move {
                web::App::new()
                    .state(ctx)
                    .state(cache_map)
                    .middleware(pipeline)
                    .configure(|cfg| {
                        crate::router::register_all(cfg);
                    })
            }
        })
        .await;

        TestApp {
            server: srv,
            default_headers: self.default_headers,
        }
    }
}
