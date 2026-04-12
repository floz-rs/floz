//! Application builder — the main entry point for floz apps.
//!
//! # Example
//! ```ignore
//! use floz::prelude::*;
//!
//! #[route(get: "/health", tag: "System", desc: "Health check")]
//! async fn health() -> HttpResponse {
//!     HttpResponse::Ok().body("OK")
//! }
//!
//! #[ntex::main]
//! async fn main() -> std::io::Result<()> {
//!     App::new().run().await
//! }
//! ```

use crate::app::context::AppContext;
use crate::config::Config;
use crate::middleware::pipeline::{EmptyStack, FlozPipeline, Process};
use crate::server::ServerConfig;
use ntex::web::{self, HttpServer};
use std::io;
use tracing::info;

/// The floz application builder.
///
/// `M` tracks the middleware stack at compile time — users never write it.
/// Rust infers the full type from the builder chain.
///
/// # Minimal
/// ```ignore
/// App::new().run().await
/// ```
///
/// # With options
/// ```ignore
/// App::new()
///     .config(Config::from_env())
///     .server(
///         ServerConfig::new()
///             .with_default_port(8080)
///             .with_middleware(Cors::permissive())
///             .with_middleware(RequestLogger)
///     )
///     .on_boot(|ctx| {
///         info!("Database ready");
///     })
///     .run()
///     .await
/// ```
pub struct App<M = EmptyStack> {
    config: Option<Config>,
    server_config: ServerConfig<M>,
    extensions: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
    on_start: Option<Box<dyn FnOnce(AppContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send>>,
    #[cfg(feature = "worker")]
    background_worker_concurrency: Option<usize>,
    schedules: Vec<(u64, Box<dyn Fn(AppContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>)>,
}

impl App<EmptyStack> {
    /// Create a new App builder with defaults.
    pub fn new() -> Self {
        Self {
            config: None,
            server_config: ServerConfig::default(),
            extensions: std::collections::HashMap::new(),
            on_start: None,
            #[cfg(feature = "worker")]
            background_worker_concurrency: None,
            schedules: Vec::new(),
        }
    }
}

impl Default for App<EmptyStack> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> App<M> {
    /// Set a custom Config. If not called, Config::from_env() is used.
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Set custom server bind configuration (with middleware stack).
    ///
    /// The middleware type changes — this returns `App<N>` where `N`
    /// is whatever the ServerConfig carries.
    pub fn server<N>(self, server_config: ServerConfig<N>) -> App<N> {
        App {
            config: self.config,
            server_config,
            extensions: self.extensions,
            on_start: self.on_start,
            #[cfg(feature = "worker")]
            background_worker_concurrency: self.background_worker_concurrency,
            schedules: self.schedules,
        }
    }

    /// Register a custom state extension to be available in route handlers.
    ///
    /// Available in handlers via `state.ext::<T>()`.
    pub fn with<T: Send + Sync + 'static>(mut self, data: T) -> Self {
        self.extensions.insert(std::any::TypeId::of::<T>(), Box::new(data));
        self
    }

    /// Run a background task worker in-process alongside the HTTP server.
    #[cfg(feature = "worker")]
    pub fn with_worker(mut self, concurrency: usize) -> Self {
        self.background_worker_concurrency = Some(concurrency);
        self
    }

    /// Register an async callback that runs after the database pool is ready.
    ///
    /// Use this for migrations, table creation, seeding — anything that
    /// needs the shared state before the server starts accepting requests.
    ///
    /// ```ignore
    /// App::new()
    ///     .on_start(|ctx| async move {
    ///         Note::create_table(&ctx.db()).await.unwrap();
    ///     })
    ///     .run()
    ///     .await
    /// ```
    pub fn on_start<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(AppContext) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let wrapped = Box::new(move |ctx: AppContext| {
            Box::pin(f(ctx)) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });
        self.on_start = Some(wrapped);
        self
    }

    /// Register a recurring background task that runs at a specific interval.
    ///
    /// The task runs continuously in an async Tokio task alongside the HTTP server.
    /// `interval_secs` defines the exact delay between executions (handles drift).
    ///
    /// ```ignore
    /// App::new()
    ///     .schedule(60, |ctx| async move {
    ///         info!("Running background check...");
    ///     })
    /// ```
    pub fn schedule<F, Fut>(mut self, interval_secs: u64, f: F) -> Self
    where
        F: Fn(AppContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let wrapped = Box::new(move |ctx: AppContext| {
            Box::pin(f(ctx)) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });
        self.schedules.push((interval_secs, wrapped));
        self
    }
}

impl<M: Process> App<M> {
    /// Build and run the application.
    ///
    /// This initializes the database pool, logging, and starts
    /// the HTTP server with all auto-discovered routes, OpenAPI
    /// spec generation, and Swagger UI.
    ///
    /// All handlers annotated with `#[route(...)]` are auto-discovered
    /// and registered — no manual wiring needed.
    ///
    /// The middleware pipeline is fully monomorphized — zero overhead.
    pub async fn run(self) -> io::Result<()> {
        // Initialize config
        let config = self.config.unwrap_or_else(Config::from_env);

        // Initialize logging
        #[cfg(feature = "logger")]
        crate::logger::init_tracing();

        info!("🚀 floz starting...");
        info!("   Environment: {}", config.server_env);

        // Initialize AppContext
        let ctx = AppContext::init(self.extensions).await;
        info!("   Database pool initialized");

        // Log auto-discovered routes
        let route_count = inventory::iter::<crate::router::RouteEntry>.into_iter().count();
        info!("   Auto-discovered {} route(s)", route_count);

        // Print route table if requested or in dev mode
        if std::env::var("FLOZ_PRINT_ROUTES").is_ok() || config.is_dev() {
            crate::router::print_route_table();
        }

        // Run async start hook (migrations, seeds, etc.)
        if let Some(on_start) = self.on_start {
            on_start(ctx.clone()).await;
        }

        // Start scheduled recurring tasks
        for (interval_secs, task_fn) in self.schedules {
            let ctx_clone = ctx.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
                // Wait exact interval before initial execution
                interval.tick().await;
                loop {
                    interval.tick().await;
                    task_fn(ctx_clone.clone()).await;
                }
            });
        }

        // Setup Declarative Caching Sweeper
        #[cfg(all(feature = "postgres", feature = "worker"))]
        if let Some(ref cache) = ctx.cache {
            let db_pool = ctx.db_pool.clone();
            
            // 2. Spawn the Sweeper Poller Background Tokio task
            let cache_clone = cache.clone();
            tokio::spawn(async move {
                // 1. Ensure the outbox table exists safely
                let _ = floz_orm::sqlx::query(
                    "CREATE TABLE IF NOT EXISTS _floz_cache_outbox (
                        id BIGSERIAL PRIMARY KEY,
                        entity_table VARCHAR(255) NOT NULL,
                        entity_id VARCHAR(255) NOT NULL,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )"
                ).execute(&*db_pool).await;

                tracing::info!("🧹 Cache Invalidation Sweeper started (500ms intervals)");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
                    let result: Result<Vec<(i64, String, String)>, _> = floz_orm::sqlx::query_as(
                        "SELECT id, entity_table, entity_id FROM _floz_cache_outbox ORDER BY id ASC LIMIT 500"
                    )
                        .fetch_all(&*db_pool)
                        .await;

                    match result {
                        Ok(events) => {
                            if !events.is_empty() {
                                for (_id, table, entity_id) in &events {
                                    // Drop specific row tags (e.g. users:5)
                                    let tag = format!("{}:{}", table, entity_id);
                                    let _ = cache_clone.drop_by_tag(&tag).await;
                                    // Drop the entire table tag list (e.g. users)
                                    let _ = cache_clone.drop_by_tag(table).await;
                                }
                                
                                // Safely erase swept items
                                if let Some((last_id, _, _)) = events.last() {
                                    let _ = floz_orm::sqlx::query("DELETE FROM _floz_cache_outbox WHERE id <= $1")
                                        .bind(last_id)
                                        .execute(&*db_pool)
                                        .await;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Cache outbox sweeper database poll error: {}", e);
                        }
                    }
                }
            });
        }

        #[cfg(feature = "worker")]
        if let Some(concurrency) = self.background_worker_concurrency {
            if let Some(redis_url) = config.redis_url.clone() {
                let broker = std::sync::Arc::new(crate::worker::RedisBroker::new(&redis_url).await.expect("Failed to connect worker to Redis"));
                let worker = crate::worker::Worker::new(ctx.clone(), broker).concurrency(concurrency);
                tokio::spawn(async move {
                    if let Err(e) = worker.run().await {
                        tracing::error!("Background worker failed: {:?}", e);
                    }
                });
            } else {
                tracing::warn!("App::with_worker() was called but REDIS_URL is not set. Background workers are disabled.");
            }
        }

        // Get addr before moving middlewares out
        let addr = self.server_config.get_socket_addr();
        let max_payload_size = self.server_config.max_payload_size;
        let global_rate_limit = self.server_config.global_rate_limit.clone();
        
        let pipeline = FlozPipeline::new(self.server_config.middlewares);

        let cache_route_map = std::sync::Arc::new(crate::router::build_cache_route_map());
        if !cache_route_map.is_empty() {
            info!("   Cache-enabled routes: {}", cache_route_map.len());
        }

        // Build the security route map for AuthMiddleware lookups
        let security_route_map = std::sync::Arc::new(crate::router::build_security_route_map());
        if !security_route_map.is_empty() {
            info!("   Secured routes: {}", security_route_map.len());
        }

        // Build the rate limit route map for RateLimitMiddleware
        let rate_limit_route_map = std::sync::Arc::new(crate::router::build_rate_limit_route_map(global_rate_limit));
        if !rate_limit_route_map.is_empty() {
            info!("   Rate limited routes: {}", rate_limit_route_map.len());
        }

        info!("   Binding to {}", addr);

        // Generate OpenAPI spec once and serialize it
        let openapi_spec = crate::router::generate_openapi();
        let openapi_json = match openapi_spec.to_pretty_json() {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize OpenAPI spec for docs: {}", e);
                "{}".to_string()
            }
        };

        let server = HttpServer::new(move || {
            let openapi_json_worker = openapi_json.clone();
            let pipeline = pipeline.clone();
            let ctx = ctx.clone();
            let cache_map = cache_route_map.clone();
            let security_map = security_route_map.clone();
            let rate_limit_map = rate_limit_route_map.clone();
            let read_ctx = ctx.clone();
            async move {
                let app = web::App::new()
                    .state(ctx)
                    .state(cache_map)
                    .state(security_map)
                    .state(rate_limit_map)
                    .state(ntex::web::types::PayloadConfig::new(max_payload_size))
                    .state(ntex::web::types::JsonConfig::default().limit(max_payload_size))
                    .middleware(pipeline);

                #[cfg(feature = "compression")]
                let mut app = app.middleware(ntex::web::middleware::Compress::default());

                #[cfg(not(feature = "compression"))]
                let mut app = app;

            app = app.route("/api-docs/openapi.json", web::get().to(move || {
                let json_data = openapi_json_worker.clone();
                async move {
                    web::HttpResponse::Ok()
                        .content_type("application/json")
                        .body(json_data)
                }
            }));

            // Serve bundled Swagger UI assets (no external CDN dependency)
            app = app.route("/api-docs/swagger-ui-bundle.js", web::get().to(|| {
                async {
                    web::HttpResponse::Ok()
                        .content_type("application/javascript")
                        .body(crate::router::SWAGGER_UI_BUNDLE_JS)
                }
            }));

            app = app.route("/api-docs/swagger-ui.css", web::get().to(|| {
                async {
                    web::HttpResponse::Ok()
                        .content_type("text/css")
                        .body(crate::router::SWAGGER_UI_CSS)
                }
            }));

            let swagger_html = crate::router::SWAGGER_UI_HTML_TEMPLATE
                .replace("{DARK_THEME_CSS}", include_str!("../swagger-dark-theme.css"));
            
            app = app.route("/ui", web::get().to(move || {
                let html = swagger_html.clone();
                async move {
                    web::HttpResponse::Ok()
                        .content_type("text/html")
                        .body(html)
                }
            }));

            // Setup built-in /health (Liveness) and /readiness (Dependencies check)
            // if we wanted to make them overridable, we could check if they exist, but Ntex routes evaluate in order.
            // By putting them *after* the auto-discovery, user routes take precedence!
            // Actually, we'll put them before `configure()`, but Ntex routes are evaluated in order of registration.
            // Wait, to allow user override, we should just let them register them. Let's just add them as basic routes.
            app = app.route("/health", web::get().to(|| async {
                web::HttpResponse::Ok().json(&serde_json::json!({ "status": "ok" }))
            }));

            app = app.route("/readiness", web::get().to(move || {
                let context = read_ctx.clone();
                async move {
                    let mut ready = true;
                    // Check DB
                    #[cfg(any(feature = "postgres", feature = "sqlite"))]
                    if context.db_pool.is_closed() {
                        ready = false;
                    }

                    // Check Redis
                    #[cfg(feature = "worker")]
                    if let Some(ref cache) = context.cache {
                        let mut conn = cache.connection();
                        use redis::AsyncCommands;
                        let ping: redis::RedisResult<String> = conn.ping().await;
                        if ping.is_err() {
                            ready = false;
                        }
                    }

                    // Build response payload, injecting pool stats directly at root level
                    let mut payload = serde_json::json!({ 
                        "status": if ready { "ready" } else { "unavailable" }
                    });

                    #[cfg(any(feature = "postgres", feature = "sqlite"))]
                    if let serde_json::Value::Object(ref mut map) = payload {
                        map.insert("db_size".to_string(), serde_json::json!(context.db_pool.size()));
                        map.insert("db_idle".to_string(), serde_json::json!(context.db_pool.num_idle()));
                    }

                    if ready {
                        web::HttpResponse::Ok().json(&payload)
                    } else {
                        web::HttpResponse::ServiceUnavailable().json(&payload)
                    }
                }
            }));

            // Provide the central WebSocket Channels bridge endpoint
            app = app.route("/ws/channels", web::get().to(crate::web::channels::ws_channels_handler));

            // Auto-register all #[route(...)] handlers
            // Ntex evaluates routes in the order they are added.
            // But `/health` above might conflict. To ensure user overrides work, 
            // wait, ntex allows overriding if we don't strictly care about order unless the exact same path.
            // Actually, just append them.
            app = app.configure(|cfg| {
                crate::router::register_all(cfg);
            });

            app
            }
        });

        // Setup TLS if configured via .env
        let global_config = crate::config::Config::global();
        #[cfg(feature = "rustls")]
        let mut tls_config = None;
        
        #[cfg(feature = "rustls")]
        if let (Some(cert_path), Some(key_path)) = (&global_config.tls_cert_path, &global_config.tls_key_path) {
            let cert_file = &mut std::io::BufReader::new(std::fs::File::open(cert_path)
                .unwrap_or_else(|e| panic!("TLS_CERT_PATH not found: {e}")));
            let key_file = &mut std::io::BufReader::new(std::fs::File::open(key_path)
                .unwrap_or_else(|e| panic!("TLS_KEY_PATH not found: {e}")));
            
            let cert_chain = rustls_pemfile::certs(cert_file).collect::<Result<Vec<_>, _>>().unwrap();
            
            // Try PKCS8
            let mut keys = rustls_pemfile::pkcs8_private_keys(key_file).filter_map(Result::ok).collect::<Vec<_>>();
            let key = if let Some(key) = keys.pop() {
                rustls::pki_types::PrivateKeyDer::Pkcs8(key)
            } else {
                // Try RSA
                let key_file2 = &mut std::io::BufReader::new(std::fs::File::open(key_path).unwrap());
                let mut rsa_keys = rustls_pemfile::rsa_private_keys(key_file2).filter_map(Result::ok).collect::<Vec<_>>();
                let key = rsa_keys.pop().expect("Failed to locate any valid private keys in TLS_KEY_PATH");
                rustls::pki_types::PrivateKeyDer::Pkcs1(key)
            };
            
            let config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert_chain, key)
                .expect("Failed to construct rustls ServerConfig");
            tls_config = Some(config);
            tracing::info!("   TLS/HTTPS enabled using rustls");
        }

        let server_builder = server;
        
        #[cfg(feature = "rustls")]
        let server_builder = if let Some(tls) = tls_config {
            server_builder.bind_rustls(addr, &tls)?
        } else {
            server_builder.bind(addr)?
        };

        #[cfg(not(feature = "rustls"))]
        let server_builder = server_builder.bind(addr)?;
        
        // Wait up to X seconds for gracefully draining in-flight requests on Ctrl+C/SIGTERM
        let server_builder = server_builder.shutdown_timeout(ntex::time::Seconds(self.server_config.shutdown_timeout as u16));

        server_builder.run().await
    }
}
