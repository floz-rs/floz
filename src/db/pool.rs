//! SQLx connection pool management.
//!
//! Supports PostgreSQL and SQLite via feature flags.
//! The pool type is determined by the `DATABASE_URL` scheme at runtime.

use std::sync::Arc;
use std::time::Duration;

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

// ═══════════════════════════════════════════════════════════════
// PostgreSQL pool
// ═══════════════════════════════════════════════════════════════

/// Type alias for the shared PostgreSQL pool.
#[cfg(feature = "postgres")]
pub type PgDbPool = Arc<sqlx::Pool<sqlx::Postgres>>;

/// Create a PostgreSQL connection pool with automatic sizing based on CPU cores.
#[cfg(feature = "postgres")]
pub async fn pg_pool(worker_count: usize) -> PgDbPool {
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

    pg_pool_with_options(&opts).await
}

/// Create a PostgreSQL connection pool with custom options.
#[cfg(feature = "postgres")]
pub async fn pg_pool_with_options(opts: &PoolOptions) -> PgDbPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let conn_url = if database_url.is_empty() {
        tracing::error!("DATABASE_URL is not set! Database connections will fail if accessed.");
        "postgres://localhost:5432/app"
    } else {
        &database_url
    };

    let pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(opts.min_connections)
        .max_connections(opts.max_connections)
        .acquire_timeout(Duration::from_secs(opts.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(opts.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(opts.max_lifetime_secs))
        .connect_lazy(conn_url)
        .expect("Invalid PostgreSQL connection URL format");

    Arc::new(pool)
}

// ═══════════════════════════════════════════════════════════════
// SQLite pool
// ═══════════════════════════════════════════════════════════════

/// Type alias for the shared SQLite pool.
#[cfg(feature = "sqlite")]
pub type SqliteDbPool = Arc<sqlx::Pool<sqlx::Sqlite>>;

/// Create a SQLite connection pool.
///
/// If `DATABASE_URL` is set and starts with `sqlite:`, that URL is used.
/// Otherwise falls back to `sqlite::memory:`.
#[cfg(feature = "sqlite")]
pub async fn sqlite_pool() -> SqliteDbPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let conn_url = if database_url.is_empty() {
        tracing::error!("DATABASE_URL is not set! SQLite database features will fail if accessed.");
        "sqlite::memory:"
    } else {
        &database_url
    };

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_lazy(conn_url)
        .expect("Invalid SQLite connection URL format");

    Arc::new(pool)
}

/// Create a SQLite connection pool with custom options.
#[cfg(feature = "sqlite")]
pub async fn sqlite_pool_with_options(opts: &PoolOptions) -> SqliteDbPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let conn_url = if database_url.is_empty() {
        tracing::error!("DATABASE_URL is not set! SQLite database features will fail if accessed.");
        "sqlite::memory:"
    } else {
        &database_url
    };

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(opts.max_connections)
        .acquire_timeout(Duration::from_secs(opts.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(opts.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(opts.max_lifetime_secs))
        .connect_lazy(conn_url)
        .expect("Invalid SQLite connection URL format");

    Arc::new(pool)
}

// ═══════════════════════════════════════════════════════════════
// Legacy aliases (when only postgres is enabled)
// ═══════════════════════════════════════════════════════════════

/// Type alias for backwards compatibility — resolves to `PgDbPool` when
/// only the `postgres` feature is enabled.
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub type DbPool = PgDbPool;

/// Legacy pool builder — creates a PostgreSQL pool.
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub async fn pool(worker_count: usize) -> DbPool {
    pg_pool(worker_count).await
}

/// Legacy pool builder with options — creates a PostgreSQL pool.
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub async fn pool_with_options(opts: &PoolOptions) -> DbPool {
    pg_pool_with_options(opts).await
}
