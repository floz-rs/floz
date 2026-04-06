//! Abstract broker and Redis implementation for distributing tasks.

use crate::worker::message::TaskMessage;
use crate::worker::TaskError;
use redis::AsyncCommands;

/// A Celery-style Redis task broker using LPUSH and BRPOP.
#[derive(Clone)]
pub struct RedisBroker {
    conn: redis::aio::MultiplexedConnection,
}

impl RedisBroker {
    pub async fn new(url: &str) -> Result<Self, TaskError> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn })
    }

    fn queue_key(queue: &str) -> String {
        format!("floz:queue:{}", queue)
    }

    pub async fn enqueue(&self, msg: &TaskMessage) -> Result<(), TaskError> {
        let mut conn = self.conn.clone();
        let payload = serde_json::to_string(msg)?;
        let key = Self::queue_key(&msg.queue);

        // Enqueue from the left
        conn.lpush::<_, _, ()>(&key, payload).await?;
        Ok(())
    }

    pub async fn dequeue(
        &self,
        queues: &[String],
        _timeout_secs: usize, // No longer blocking via redis natively
    ) -> Result<Option<TaskMessage>, TaskError> {
        let mut conn = self.conn.clone();
        
        // RPOP pops from the right, one queue at a time
        for q in queues {
            let key = Self::queue_key(q);
            let reply: Option<String> = conn.rpop(&key, None).await?;

            if let Some(payload) = reply {
                let msg: TaskMessage = serde_json::from_str(&payload)?;
                return Ok(Some(msg));
            }
        }
        
        // If all queues are empty, wait to prevent CPU spin
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(None)
    }
}
