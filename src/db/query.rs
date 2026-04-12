//! Dynamic SQL query execution utilities.
//!
//! **⚠️ SECURITY WARNING**: These functions accept raw SQL strings without
//! parameterization. **Never** interpolate user input into the query string.
//! Use the ORM's query builder API or `Executor` trait for safe, parameterized queries.
//!
//! These functions are provided as an escape hatch for advanced use cases only.
//! Prefer Floz's type-safe `SelectQuery`, `InsertQuery`, etc. builders instead.
//!
//! All sqlx access goes through `floz_orm::sqlx`.

use crate::errors::ApiError;
use floz_orm::sqlx::{Column, Row};
use tracing::info;

#[cfg(feature = "postgres")]
use crate::db::PgDbPool;

/// Execute a query and deserialize results into type `T`.
///
/// Converts each row to a JSON object, then deserializes into the target type.
/// This is the "escape hatch" for dynamic SQL — prefer Floz for type-safe queries.
#[cfg(feature = "postgres")]
pub async fn execute_query<T>(query: String, pool: &PgDbPool) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug,
{
    match floz_orm::sqlx::query(&query).fetch_all(pool.as_ref()).await {
        Ok(rows) => {
            let mut results = Vec::new();
            for row in rows {
                let mut m = serde_json::Map::new();
                for (i, column) in row.columns().iter().enumerate() {
                    let column_name = column.name();
                    if let Ok(value) = row.try_get::<serde_json::Value, _>(i) {
                        m.insert(column_name.to_string(), value);
                    }
                }
                results.push(serde_json::Value::Object(m));
            }

            let json_string = serde_json::to_string(&results)
                .map_err(|e| ApiError::from(format!("JSON serialization error: {e}")))?;

            match serde_json::from_str::<T>(&json_string) {
                Ok(message) => Ok(message),
                Err(err) => {
                    let line = err.line();
                    let column = err.column();
                    info!("JSON parsing error at line {line}, column {column}: {err}");
                    Err(ApiError::from(err.to_string()))
                }
            }
        }
        Err(err) => Err(ApiError::from(err.to_string())),
    }
}

/// Execute a query and return results as a JSON string.
#[cfg(feature = "postgres")]
pub async fn execute_query_json(query: String, pool: &PgDbPool) -> Result<String, ApiError> {
    match floz_orm::sqlx::query(&query).fetch_all(pool.as_ref()).await {
        Ok(rows) => {
            let mut results = Vec::new();
            for row in rows {
                let mut m = serde_json::Map::new();
                for (i, column) in row.columns().iter().enumerate() {
                    let column_name = column.name();
                    if let Ok(value) = row.try_get::<serde_json::Value, _>(i) {
                        m.insert(column_name.to_string(), value);
                    }
                }
                results.push(serde_json::Value::Object(m));
            }

            serde_json::to_string(&results)
                .map_err(|e| ApiError::from(format!("JSON serialization error: {e}")))
        }
        Err(err) => Err(ApiError::from(err.to_string())),
    }
}

/// Execute a query expecting exactly one row, deserialize into type `T`.
#[cfg(feature = "postgres")]
pub async fn execute_one_query<T>(query: String, pool: &PgDbPool) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug,
{
    match floz_orm::sqlx::query(&query).fetch_one(pool.as_ref()).await {
        Ok(row) => {
            let mut m = serde_json::Map::new();
            for (i, column) in row.columns().iter().enumerate() {
                let column_name = column.name();
                if let Ok(value) = row.try_get::<serde_json::Value, _>(i) {
                    m.insert(column_name.to_string(), value);
                }
            }

            let json_value = serde_json::Value::Object(m);
            let json_string = serde_json::to_string(&json_value)
                .map_err(|e| ApiError::from(format!("JSON serialization error: {e}")))?;

            serde_json::from_str(&json_string)
                .map_err(|e| ApiError::from(format!("Deserialization error: {e}")))
        }
        Err(err) => Err(ApiError::from(err.to_string())),
    }
}
