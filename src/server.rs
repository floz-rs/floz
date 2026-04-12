//! HTTP server abstraction layer.
//!
//! `ServerConfig<M>` carries the middleware stack as a generic type parameter.
//! Each `.with_middleware()` call wraps the type — fully static dispatch.

use crate::middleware::pipeline::{AsyncMiddleware, AsyncLayer, EmptyStack, Middleware, Stack, SyncLayer};
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Server configuration builder.
///
/// The type parameter `M` tracks the middleware stack at compile time.
/// Users never need to write it — Rust infers it from the builder chain.
///
/// # Example
/// ```ignore
/// ServerConfig::new()
///     .with_default_port(8080)
///     .with_middleware(Cors::permissive())          // sync
///     .with_middleware(RequestLogger)               // sync
///     .with_async_middleware(JwtAuth::new(secret))  // async
/// ```
pub struct ServerConfig<M = EmptyStack> {
    host_env_key: String,
    port_env_key: String,
    default_host: IpAddr,
    default_port: u16,
    pub(crate) max_payload_size: usize,
    pub(crate) global_rate_limit: Option<String>,
    pub(crate) shutdown_timeout: u64,
    pub(crate) middlewares: M,
}

impl ServerConfig<EmptyStack> {
    pub fn new() -> Self {
        Self {
            host_env_key: "HOST".to_string(),
            port_env_key: "PORT".to_string(),
            default_host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            default_port: 3030,
            max_payload_size: 2 * 1024 * 1024, // 2MB default
            global_rate_limit: None,
            shutdown_timeout: 30, // 30 seconds default
            middlewares: EmptyStack,
        }
    }
}

impl Default for ServerConfig<EmptyStack> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> ServerConfig<M> {
    /// Add a **sync** middleware to the pipeline. Runs in insertion order.
    ///
    /// Use this for middleware that does NOT require I/O —
    /// CORS, tracing, compression, header injection, etc.
    ///
    /// Zero-cost: the compiler fully inlines sync middleware with no async overhead.
    ///
    /// ```ignore
    /// ServerConfig::new()
    ///     .with_middleware(Cors::permissive())
    ///     .with_middleware(RequestTrace::default())
    /// ```
    pub fn with_middleware<N: Middleware>(self, mw: N) -> ServerConfig<Stack<M, SyncLayer<N>>> {
        ServerConfig {
            host_env_key: self.host_env_key,
            port_env_key: self.port_env_key,
            default_host: self.default_host,
            default_port: self.default_port,
            max_payload_size: self.max_payload_size,
            global_rate_limit: self.global_rate_limit,
            shutdown_timeout: self.shutdown_timeout,
            middlewares: Stack {
                inner: self.middlewares,
                outer: SyncLayer(mw),
            },
        }
    }

    /// Add an **async** middleware to the pipeline. Runs in insertion order.
    ///
    /// Use this for middleware that requires I/O —
    /// database lookups, Redis cache checks, external API calls, etc.
    ///
    /// ```ignore
    /// ServerConfig::new()
    ///     .with_middleware(Cors::permissive())              // sync
    ///     .with_async_middleware(JwtAuth::new(secret))      // async
    ///     .with_async_middleware(RateLimiter::new(100))     // async
    /// ```
    pub fn with_async_middleware<N: AsyncMiddleware>(self, mw: N) -> ServerConfig<Stack<M, AsyncLayer<N>>> {
        ServerConfig {
            host_env_key: self.host_env_key,
            port_env_key: self.port_env_key,
            default_host: self.default_host,
            default_port: self.default_port,
            max_payload_size: self.max_payload_size,
            global_rate_limit: self.global_rate_limit,
            shutdown_timeout: self.shutdown_timeout,
            middlewares: Stack {
                inner: self.middlewares,
                outer: AsyncLayer(mw),
            },
        }
    }

    pub fn with_host_key(mut self, key: &str) -> Self {
        self.host_env_key = key.to_string();
        self
    }

    pub fn with_port_key(mut self, key: &str) -> Self {
        self.port_env_key = key.to_string();
        self
    }

    /// Set the graceful shutdown timeout in seconds.
    pub fn with_shutdown_timeout(mut self, seconds: u64) -> Self {
        self.shutdown_timeout = seconds;
        self
    }

    pub fn with_default_host(mut self, host: IpAddr) -> Self {
        self.default_host = host;
        self
    }

    pub fn with_default_port(mut self, port: u16) -> Self {
        self.default_port = port;
        self
    }

    pub fn get_socket_addr(&self) -> SocketAddr {
        let host = env::var(&self.host_env_key)
            .ok()
            .and_then(|h| h.parse().ok())
            .unwrap_or(self.default_host);

        let port = env::var(&self.port_env_key)
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(self.default_port);

        SocketAddr::new(host, port)
    }

    /// Set the maximum payload body size in bytes. Default is 2MB (2,097,152 bytes).
    pub fn with_max_body_size(mut self, bytes: usize) -> Self {
        self.max_payload_size = bytes;
        self
    }

    /// Set the global fallback rate limit (e.g. "100/min"). 
    /// This will be applied to all routes unless they declare a specific override.
    pub fn with_global_rate_limit(mut self, limit: &str) -> Self {
        self.global_rate_limit = Some(limit.to_string());
        self
    }
}
