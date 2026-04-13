//! Worker runtime for processing background tasks.

use crate::app::AppContext;
use crate::worker::broker::RedisBroker;
use crate::worker::message::TaskMessage;
use crate::worker::registry::find_task;
use crate::worker::retry::exponential_backoff;
use chrono::Utc;
use std::sync::Arc;
use tokio::time::sleep;

/// The worker runtime that polls the broker and executes tasks.
pub struct Worker {
    ctx: AppContext,
    broker: Arc<RedisBroker>,
    queues: Vec<String>,
    concurrency: usize,
}

impl Worker {
    /// Create a new worker instance.
    pub fn new(ctx: AppContext, broker: Arc<RedisBroker>) -> Self {
        Self {
            ctx,
            broker,
            queues: vec!["default".to_string()],
            concurrency: 1,
        }
    }

    /// Set the list of queues this worker should pull from.
    pub fn queues<I, S>(mut self, queues: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.queues = queues.into_iter().map(Into::into).collect();
        self
    }

    /// Set the number of concurrent task runners.
    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    /// Run the worker indefinitely.
    pub async fn run(self) -> std::io::Result<()> {
        tracing::info!(
            "👷 Worker started (concurrency: {}, queues: {:?})",
            self.concurrency,
            self.queues
        );

        let worker = Arc::new(self);
        let mut handles = Vec::with_capacity(worker.concurrency);

        for i in 0..worker.concurrency {
            let w = worker.clone();
            handles.push(tokio::spawn(async move {
                w.runner_loop(i).await;
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    async fn runner_loop(&self, worker_id: usize) {
        loop {
            match self.broker.dequeue(&self.queues, 5).await {
                Ok(Some(msg)) => {
                    self.process_message(worker_id, msg).await;
                }
                Ok(None) => {
                    // Timeout (5s), just loop again
                }
                Err(e) => {
                    tracing::error!("Worker {} broker error: {:?}", worker_id, e);
                    sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn process_message(&self, worker_id: usize, mut msg: TaskMessage) {
        // Handle delayed execution
        if let Some(eta) = msg.eta {
            let now = Utc::now();
            if now < eta {
                let delay = (eta - now)
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(1));

                // If the delay is short, sleep natively. Disadvantage: blocks this runner.
                // If it's long, re-enqueue it.
                // For a robust system, there should be a dedicated scheduler processing ETAs.
                // For MVP, we sleep if < 60s, else re-enqueue and sleep briefly to avoid spinning.
                if delay.as_secs() < 60 {
                    sleep(delay).await;
                } else {
                    let _ = self.broker.enqueue(&msg).await;
                    sleep(std::time::Duration::from_secs(5)).await;
                    return;
                }
            }
        }

        tracing::info!(
            "Worker {} starting task: {} [{}]",
            worker_id,
            msg.name,
            msg.id
        );

        let task_def = match find_task(&msg.name) {
            Some(def) => def,
            None => {
                tracing::error!("Unknown task type: {}", msg.name);
                return;
            }
        };

        // Call the task handler
        let result = task_def.call(self.ctx.clone(), msg.args.clone()).await;

        match result {
            Ok(_) => {
                tracing::info!(
                    "Worker {} completed task: {} [{}]",
                    worker_id,
                    msg.name,
                    msg.id
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Worker {} failed task: {} [{}] (attempt {}/{}): {:?}",
                    worker_id,
                    msg.name,
                    msg.id,
                    msg.retries + 1,
                    msg.max_retries + 1,
                    e
                );

                if msg.retries < msg.max_retries {
                    // Retry
                    msg.retries += 1;
                    let delay = exponential_backoff(msg.retries);
                    msg.eta = Some(Utc::now() + chrono::Duration::from_std(delay).unwrap());

                    if let Err(e) = self.broker.enqueue(&msg).await {
                        tracing::error!("Failed to requeue task {}: {:?}", msg.id, e);
                    }
                } else {
                    tracing::error!(
                        "Worker {} permanently failed task: {} [{}] after {} retries",
                        worker_id,
                        msg.name,
                        msg.id,
                        msg.max_retries
                    );
                }
            }
        }
    }
}
