//! Redis connection pool and cache wrapper.

use crate::config::Config;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

/// A high-level Redis cache client.
/// Provides typed get/set with JSON, and basic Redis commands.
#[derive(Clone)]
pub struct Cache {
    conn: Arc<MultiplexedConnection>,
}

impl Cache {
    /// Initialize the cache from a Redis URL.
    pub async fn new(url: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            conn: Arc::new(conn),
        })
    }

    /// Initialize the cache using `REDIS_URL` from the environment or config.
    pub async fn from_env() -> redis::RedisResult<Option<Self>> {
        let config = Config::from_env();
        if let Some(url) = config.redis_url {
            Ok(Some(Self::new(&url).await?))
        } else {
            Ok(None)
        }
    }

    /// Get a raw string value from the cache.
    pub async fn get(&self, key: &str) -> redis::RedisResult<Option<String>> {
        let mut conn = self.conn.as_ref().clone();
        conn.get(key).await
    }

    /// Get a JSON-serialized value from the cache.
    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> redis::RedisResult<Option<T>> {
        let json_str: Option<String> = self.get(key).await?;
        if let Some(s) = json_str {
            match serde_json::from_str(&s) {
                Ok(val) => Ok(Some(val)),
                Err(e) => Err(redis::RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ))),
            }
        } else {
            Ok(None)
        }
    }

    /// Set a string value in the cache with a Time-To-Live (TTL) in seconds.
    pub async fn set(&self, key: &str, value: &str, ttl_secs: u64) -> redis::RedisResult<()> {
        let mut conn = self.conn.as_ref().clone();
        conn.set_ex(key, value, ttl_secs).await
    }

    /// Set a JSON-serializable value in the cache with a TTL.
    pub async fn set_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: u64,
    ) -> redis::RedisResult<()> {
        match serde_json::to_string(value) {
            Ok(s) => self.set(key, &s, ttl_secs).await,
            Err(e) => Err(redis::RedisError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))),
        }
    }

    /// Delete a key from the cache.
    pub async fn del(&self, key: &str) -> redis::RedisResult<()> {
        let mut conn = self.conn.as_ref().clone();
        conn.del(key).await
    }

    /// Check if a key exists.
    pub async fn exists(&self, key: &str) -> redis::RedisResult<bool> {
        let mut conn = self.conn.as_ref().clone();
        conn.exists(key).await
    }

    /// Update the TTL of an existing key.
    pub async fn expire(&self, key: &str, ttl_secs: u64) -> redis::RedisResult<()> {
        let mut conn = self.conn.as_ref().clone();
        conn.expire(key, ttl_secs as i64).await
    }

    /// Atomically increment an integer value.
    pub async fn incr(&self, key: &str) -> redis::RedisResult<i64> {
        let mut conn = self.conn.as_ref().clone();
        conn.incr(key, 1).await
    }

    /// Get the remaining TTL in seconds for a key.
    pub async fn ttl(&self, key: &str) -> redis::RedisResult<i64> {
        let mut conn = self.conn.as_ref().clone();
        conn.ttl(key).await
    }

    /// Remove all keys from the current database.
    pub async fn flush(&self) -> redis::RedisResult<()> {
        let mut conn = self.conn.as_ref().clone();
        redis::cmd("FLUSHDB").query_async(&mut conn).await
    }

    /// Associate a cache key with a specific dependency tag.
    pub async fn add_tag(&self, tag: &str, cache_key: &str) -> redis::RedisResult<()> {
        let mut conn = self.conn.as_ref().clone();
        let set_key = format!("floz:cache:tags:{}", tag);
        conn.sadd(&set_key, cache_key).await
    }

    /// Drop all cache keys associated with a tag, then drop the tag itself.
    pub async fn drop_by_tag(&self, tag: &str) -> redis::RedisResult<()> {
        let mut conn = self.conn.as_ref().clone();
        let set_key = format!("floz:cache:tags:{}", tag);

        let members: Vec<String> = conn.smembers(&set_key).await?;
        if !members.is_empty() {
            // Redis DEL takes an array of keys
            conn.del::<_, ()>(&members).await?;
        }
        conn.del::<_, ()>(&set_key).await?;
        Ok(())
    }

    /// Get a cloned connection matching `MultiplexedConnection` for advanced commands.
    pub fn connection(&self) -> MultiplexedConnection {
        self.conn.as_ref().clone()
    }
}
