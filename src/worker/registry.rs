//! Dynamic registration of tasks using `inventory`.

use crate::app::AppContext;
use crate::worker::TaskError;
use std::future::Future;
use std::pin::Pin;

/// Trait implemented by the auto-generated struct for each `#[task]`.
pub trait TaskDef: Send + Sync {
    /// The unique name of the task (e.g. "send_welcome_email").
    fn name(&self) -> &'static str;

    /// The default queue for the task.
    fn default_queue(&self) -> &'static str;

    /// Maximum retries.
    fn max_retries(&self) -> u32;

    /// Handler to deserialize arguments and execute the task.
    fn call<'a>(
        &'a self,
        ctx: AppContext,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<(), TaskError>> + Send + 'a>>;
}

/// The registry entry holding a reference to a task definition.
pub struct TaskEntry {
    pub inner: &'static dyn TaskDef,
}

impl TaskEntry {
    pub const fn new(inner: &'static dyn TaskDef) -> Self {
        Self { inner }
    }
}

inventory::collect!(TaskEntry);

/// Find a task definition by name.
pub fn find_task(name: &str) -> Option<&'static dyn TaskDef> {
    for entry in inventory::iter::<TaskEntry> {
        if entry.inner.name() == name {
            return Some(entry.inner);
        }
    }
    None
}


