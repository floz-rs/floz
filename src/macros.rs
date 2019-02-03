//! Utility macros for floz.
//!
//! These macros are re-exported in the prelude for convenience.

/// Debug logging macro — only prints when `ECHO` env var is set.
///
/// Automatically pretty-prints JSON objects.
#[macro_export]
macro_rules! echo {
    ($($rest:tt)*) => {
        if std::env::var("ECHO").is_ok() {
            tracing::info!($($rest)*);
        }
    }
}

/// Shorthand for creating an `HttpResponse` with JSON content type.
///
/// ```ignore
/// res!(body)              // 200 OK
/// res!(body, 201)         // Custom status code
/// ```
#[macro_export]
macro_rules! res {
    ($body:expr) => {
        ntex::web::HttpResponse::Ok()
            .content_type("application/json")
            .body($body)
    };
    ($body:expr, $status:expr) => {
        ntex::web::HttpResponse::build(
            ntex::http::StatusCode::from_u16($status)
                .unwrap_or(ntex::http::StatusCode::OK)
        )
            .content_type("application/json")
            .body($body)
    };
}

/// Pretty-print serialization — uses `to_string_pretty` in DEV mode.
///
/// ```ignore
/// let json = pp!(&my_data).unwrap_or_default();
/// ```
#[macro_export]
macro_rules! pp {
    ($data:expr) => {{
        match std::env::var("SERVER_ENV") {
            Ok(env_value) if env_value.as_str() == "DEV" => {
                serde_json::to_string_pretty($data)
            },
            _ => {
                serde_json::to_string($data)
            }
        }
    }};
}

/// Shorthand for creating a bound SQLx query.
///
/// ```ignore
/// xquery!("SELECT * FROM users WHERE id = $1", user_id)
/// ```
#[macro_export]
macro_rules! xquery {
    ($sql:expr) => {
        sqlx::query($sql)
    };
    ($sql:expr, $($param:expr),*) => {
        sqlx::query($sql)
            $(.bind($param))*
    };
}

/// Convert a SQLx Row to a JSON map.
///
/// ```ignore
/// let map = to_json!(row);
/// ```
#[macro_export]
macro_rules! to_json {
    ($row:expr) => {{
        use serde_json::Value;
        use sqlx::{Column, Row};
        let mut map = serde_json::Map::new();
        let columns = $row.columns();
        for column in columns {
            let name = column.name();
            let value: Value = match column.type_info().name() {
                "UUID" => {
                    let val: uuid::Uuid = $row.get(name);
                    Value::String(val.to_string())
                }
                "TEXT" | "VARCHAR" => {
                    let val: String = $row.get(name);
                    Value::String(val)
                }
                "INT4" | "INT8" => {
                    let val: i64 = $row.get(name);
                    Value::Number(val.into())
                }
                "FLOAT4" | "FLOAT8" => {
                    let val: f64 = $row.get(name);
                    Value::Number(serde_json::Number::from_f64(val).unwrap_or_default())
                }
                "BOOL" => {
                    let val: bool = $row.get(name);
                    Value::Bool(val)
                }
                "TIMESTAMPTZ" | "TIMESTAMP" => {
                    let val: chrono::DateTime<chrono::Utc> = $row.get(name);
                    Value::String(val.to_rfc3339())
                }
                _ => Value::Null
            };
            map.insert(name.to_string(), value);
        }
        map
    }};
}
