//! Background task worker infrastructure for floz.
//!
//! Provides a Celery-like task queue using Redis.
//! Tasks are defined via the `#[task]` macro, dispatched using `.dispatch()`,
//! and executed by spinning up a `Worker` instance.

mod broker;
mod message;
mod registry;
pub mod retry;
mod runner;

pub use broker::RedisBroker;
pub use message::TaskMessage;
pub use registry::{TaskDef, TaskEntry};
pub use runner::Worker;

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Redis error: {0}")]
    Broker(#[from] redis::RedisError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Task panicked or aborted unexpectedly")]
    Panic,
    #[error("General task error: {0}")]
    General(String),
}
