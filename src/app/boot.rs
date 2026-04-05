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

        #[cfg(feature = "worker")]
        if let Some(concurrency) = self.background_worker_concurrency {
            let redis_url = config.redis_url.clone().expect("REDIS_URL must be set to run background workers");
            let broker = std::sync::Arc::new(crate::worker::RedisBroker::new(&redis_url).await.expect("Failed to connect worker to Redis"));
            let worker = crate::worker::Worker::new(ctx.clone(), broker).concurrency(concurrency);
            tokio::spawn(async move {
                if let Err(e) = worker.run().await {
                    tracing::error!("Background worker failed: {:?}", e);
                }
            });
        }

        // Get addr before moving middlewares out
        let addr = self.server_config.get_socket_addr();
        let pipeline = FlozPipeline::new(self.server_config.middlewares);

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

        HttpServer::new(move || {
            let openapi_json_worker = openapi_json.clone();
            let pipeline = pipeline.clone();
            let ctx = ctx.clone();
            async move {
                let mut app = web::App::new()
                    .state(ctx)
                    // Apply the floz middleware pipeline — fully inlined
                    .middleware(pipeline);

            // Mount Swagger UI and OpenAPI JSON endpoints
            app = app.route("/api-docs/openapi.json", web::get().to(move || {
                let json_data = openapi_json_worker.clone();
                async move {
                    web::HttpResponse::Ok()
                        .content_type("application/json")
                        .body(json_data)
                }
            }));
            
            app = app.route("/docs", web::get().to(|| async {
                web::HttpResponse::Ok()
                    .content_type("text/html")
                    .body(crate::router::SWAGGER_UI_HTML)
            }));

            // Auto-register all #[route(...)] handlers
            app = app.configure(|cfg| {
                crate::router::register_all(cfg);
            });

            app
            }
        })
        .bind(addr)?
        .run()
        .await
    }
}
