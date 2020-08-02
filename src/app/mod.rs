//! Application bootstrap and lifecycle management.
//!
//! Provides the `App` builder and `AppContext` (shared state).

mod boot;
mod context;

pub use boot::App;
pub use context::AppContext;
