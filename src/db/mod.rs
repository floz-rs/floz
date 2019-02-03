//! Database connection layer for floz.
//!
//! Provides SQLx connection pool management and dynamic query execution.

mod pool;
mod query;

pub use pool::{DbPool, pool, pool_with_options, PoolOptions};
pub use query::{execute_query, execute_query_json, execute_one_query};
