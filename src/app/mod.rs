//! Application bootstrap and lifecycle management.
//!
//! Provides the `App` builder and `AppContext` (shared state).

mod boot;
mod context;
mod req;

pub use boot::App;
pub use context::AppContext;
pub use req::{AuthInfo, Context, RequestContext};
