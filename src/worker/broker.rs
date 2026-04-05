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
        timeout_secs: usize,
    ) -> Result<Option<TaskMessage>, TaskError> {
        let mut conn = self.conn.clone();
        
        // Build the BRPOP queue list
        let mut keys = Vec::with_capacity(queues.len());
        for q in queues {
            keys.push(Self::queue_key(q));
        }

        // BRPOP removes and returns the last element of the list (pop from right)
        // Format of reply: (queue_name, value)
        let reply: Option<(String, String)> = conn.brpop(&keys, timeout_secs as f64).await?;

        if let Some((_, payload)) = reply {
            let msg: TaskMessage = serde_json::from_str(&payload)?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}
