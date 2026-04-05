//! Task Message format specifying how tasks are serialized over Redis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The internal payload dispatched to the broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    /// A unique identifier for this task instance.
    pub id: String,
    /// The name of the registered task (e.g. "send_welcome_email").
    pub name: String,
    /// The target queue to run this on.
    pub queue: String,
    /// JSON array of arguments for the task function.
    pub args: serde_json::Value,
    /// Current retry count.
    pub retries: u32,
    /// Maximum allowed retries before failing completely.
    pub max_retries: u32,
    /// When the task was published.
    pub created_at: DateTime<Utc>,
    /// Optional timestamp to delay execution until (schedule/delay).
    pub eta: Option<DateTime<Utc>>,
}

impl TaskMessage {
    pub fn new(
        name: impl Into<String>,
        queue: impl Into<String>,
        args: serde_json::Value,
        max_retries: u32,
    ) -> Self {
        Self {
            id: format!("task_{}", uuid::Uuid::new_v4().simple()),
            name: name.into(),
            queue: queue.into(),
            args,
            retries: 0,
            max_retries,
            created_at: Utc::now(),
            eta: None,
        }
    }

    pub fn with_eta(mut self, eta: DateTime<Utc>) -> Self {
        self.eta = Some(eta);
        self
    }
}
