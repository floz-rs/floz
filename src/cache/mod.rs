//! Redis connection and caching module for floz.
//!
//! Provides a high-level `Cache` wrapper around `redis::aio::MultiplexedConnection`
//! with built-in JSON serialization and TTL management.

mod pool;

pub use pool::Cache;
