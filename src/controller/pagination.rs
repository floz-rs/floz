//! Pagination parameters for list endpoints.
//!
//! Extracted from core/types/pagination.rs.

use serde::{Deserialize, Serialize};

/// Pagination parameters parsed from query strings.
///
/// # Usage in handlers
/// ```ignore
/// #[get("/users")]
/// async fn list_users(params: Query<PaginationParams>) -> impl Responder {
///     let page = params.into_inner();
///     // page.limit, page.offset, page.order_by
/// }
/// ```
///
/// # URL examples
/// - `/users?limit=20&offset=0&order_by=created`
/// - `/users?limit=10&offset=40&order_by=name&search=alice`
/// - `/users?limit=10&offset=0&order_by=id&filter=status:active,role:admin`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginationParams {
    /// Maximum number of records to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: u32,

    /// Number of records to skip (default: 0)
    #[serde(default)]
    pub offset: u32,

    /// Column to order by (default: "created")
    #[serde(default = "default_order_by")]
    pub order_by: String,

    /// Filter string: `col1:val1,col2:val2`
    #[serde(default)]
    pub filter: String,

    /// Search string: `val` or `col1,col2:val`
    #[serde(default)]
    pub search: String,

    /// Relationships to conditionally expand: `user_roles,posts`
    #[serde(default)]
    pub preload: Option<String>,

    /// Internal: table name (set by the framework, not by the client)
    #[serde(skip_deserializing)]
    pub table: String,

    /// Internal: module name
    #[serde(skip_deserializing)]
    pub module_name: String,

    /// Internal: record ID for single-record lookups
    #[serde(skip_deserializing)]
    pub id: String,
}

fn default_limit() -> u32 {
    10
}
fn default_order_by() -> String {
    "created".to_string()
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: 10,
            offset: 0,
            order_by: "created".to_string(),
            filter: String::new(),
            search: String::new(),
            preload: None,
            table: String::new(),
            module_name: String::new(),
            id: String::new(),
        }
    }
}

impl PaginationParams {
    /// Derive the table name from a Rust type's name.
    pub fn table_name<T>() -> String {
        std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or("UNKNOWN")
            .to_string()
    }

    /// Create PaginationParams for a specific model and module.
    pub fn for_model<T>(id: String, module_name: &str) -> Self {
        Self {
            table: Self::table_name::<T>(),
            module_name: module_name.to_string(),
            id,
            ..Default::default()
        }
    }

    /// Clone with updated table/module info for a specific model.
    pub fn with_model<T>(&self, module_name: &str) -> Self {
        Self {
            limit: self.limit,
            offset: self.offset,
            order_by: self.order_by.clone(),
            filter: self.filter.clone(),
            search: self.search.clone(),
            preload: self.preload.clone(),
            table: Self::table_name::<T>(),
            module_name: module_name.to_string(),
            id: String::new(),
        }
    }
}
