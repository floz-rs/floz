//! SQLx PostgreSQL connection pool management.

use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;

/// Type alias for the shared database pool.
pub type DbPool = Arc<Pool<Postgres>>;

/// Options for configuring the database connection pool.
pub struct PoolOptions {
    pub min_connections: u32,
    pub max_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self {
            min_connections: 4,
            max_connections: 10,
            acquire_timeout_secs: 20,
            idle_timeout_secs: 300,
            max_lifetime_secs: 900,
        }
    }
}

/// Create a database connection pool with automatic sizing based on CPU cores.
///
/// # Arguments
/// * `worker_count` — Number of workers to size the pool for.
///   Pass `0` to auto-detect from CPU count.
pub async fn pool(worker_count: usize) -> DbPool {
    let count = if worker_count == 0 {
        std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1)
    } else {
        worker_count
    };

    let opts = PoolOptions {
        max_connections: count.max(10) as u32,
        ..Default::default()
    };

    pool_with_options(&opts).await
}

/// Create a database connection pool with custom options.
pub async fn pool_with_options(opts: &PoolOptions) -> DbPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/app".to_string());

    let pg_pool = PgPoolOptions::new()
        .min_connections(opts.min_connections)
        .max_connections(opts.max_connections)
        .acquire_timeout(Duration::from_secs(opts.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(opts.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(opts.max_lifetime_secs))
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    Arc::new(pg_pool)
}
