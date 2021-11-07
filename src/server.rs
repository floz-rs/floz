//! HTTP server abstraction layer.
//!
//! `ServerConfig<M>` carries the middleware stack as a generic type parameter.
//! Each `.with_middleware()` call wraps the type — fully static dispatch.

use crate::middleware::pipeline::{EmptyStack, Middleware, Stack};
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
///     .with_middleware(Cors::permissive())
///     .with_middleware(RequestLogger)
/// ```
pub struct ServerConfig<M = EmptyStack> {
    host_env_key: String,
    port_env_key: String,
    default_host: IpAddr,
    default_port: u16,
    pub(crate) middlewares: M,
}

impl ServerConfig<EmptyStack> {
    pub fn new() -> Self {
        Self {
            host_env_key: "HOST".to_string(),
            port_env_key: "PORT".to_string(),
            default_host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            default_port: 3030,
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
    /// Add a middleware to the pipeline. Runs in insertion order.
    ///
    /// Each call wraps the stack type — zero-cost static dispatch.
    ///
    /// ```ignore
    /// ServerConfig::new()
    ///     .with_middleware(Cors::permissive())       // Stack<EmptyStack, Cors>
    ///     .with_middleware(AuthMiddleware::new("x"))  // Stack<Stack<EmptyStack, Cors>, Auth>
    ///     .with_middleware(RequestLogger)             // Stack<...>
    /// ```
    pub fn with_middleware<N: Middleware>(self, mw: N) -> ServerConfig<Stack<M, N>> {
        ServerConfig {
            host_env_key: self.host_env_key,
            port_env_key: self.port_env_key,
            default_host: self.default_host,
            default_port: self.default_port,
            middlewares: Stack {
                inner: self.middlewares,
                outer: mw,
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
}
