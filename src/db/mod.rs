//! Database connection layer for floz.
//!
//! Provides SQLx connection pool management and dynamic query execution.
//! Enable the `postgres` and/or `sqlite` features to activate the relevant pools.

#[cfg(any(feature = "postgres", feature = "sqlite"))]
mod pool;
#[cfg(feature = "postgres")]
mod query;

#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub use pool::PoolOptions;

// PostgreSQL-specific exports
#[cfg(feature = "postgres")]
pub use pool::{pg_pool, pg_pool_with_options, PgDbPool};

// SQLite-specific exports
#[cfg(feature = "sqlite")]
pub use pool::{sqlite_pool, sqlite_pool_with_options, SqliteDbPool};

// Legacy aliases (postgres-only mode)
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub use pool::{pool, pool_with_options, DbPool};

// Dynamic query functions (postgres only for now)
#[cfg(feature = "postgres")]
pub use query::{execute_one_query, execute_query, execute_query_json};
