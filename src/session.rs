#[cfg(feature = "worker")]
use crate::cache::Cache;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

#[cfg(feature = "worker")]
use redis::AsyncCommands;

/// The default Prefix for session keys in Redis
pub const SESSION_KEY_PREFIX: &str = "floz:session:";
/// The default TTL for a session in Redis (7 days)
pub const DEFAULT_SESSION_TTL: Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);

/// A lightweight request-scoped manager for manipulating the user's Session state.
/// It wraps the global Redis cache and seamlessly targets the current user's session ID.
#[derive(Clone)]
pub struct SessionStore<'a> {
    pub session_id: String,
    #[cfg(feature = "worker")]
    cache: Option<&'a Cache>,
}

impl<'a> SessionStore<'a> {
    #[cfg(feature = "worker")]
    pub fn new(session_id: String, cache: Option<&'a Cache>) -> Self {
        Self { session_id, cache }
    }

    #[cfg(not(feature = "worker"))]
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }

    /// Retrieve a generic JSON value from the Redis session store.
    #[cfg(feature = "worker")]
    pub async fn get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, crate::errors::ApiError> {
        let Some(cache) = self.cache else {
            tracing::warn!("SessionStore::get called but Redis is not configured");
            return Ok(None);
        };

        let mut conn = cache.connection();
        let cache_key = format!("{}{}:{}", SESSION_KEY_PREFIX, self.session_id, key);

        // Touch the overall ttl to keep session alive
        let _: () = conn
            .expire(&cache_key, DEFAULT_SESSION_TTL.as_secs() as i64)
            .await
            .unwrap_or(());

        let result: Option<String> = conn
            .get(&cache_key)
            .await
            .map_err(|e| crate::errors::ApiError::internal(format!("Redis get error: {}", e)))?;

        if let Some(json_str) = result {
            let deserialized = serde_json::from_str(&json_str).map_err(|e| {
                crate::errors::ApiError::internal(format!("Session deserialize error: {}", e))
            })?;
            return Ok(Some(deserialized));
        }

        Ok(None)
    }

    #[cfg(not(feature = "worker"))]
    pub async fn get<T: DeserializeOwned>(
        &self,
        _key: &str,
    ) -> Result<Option<T>, crate::errors::ApiError> {
        tracing::warn!("SessionStore requires the `worker` feature for Redis");
        Ok(None)
    }

    /// Serialize and insert a generic value into the Redis session store.
    #[cfg(feature = "worker")]
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), crate::errors::ApiError> {
        let Some(cache) = self.cache else {
            tracing::warn!("SessionStore::set called but Redis is not configured");
            return Ok(());
        };

        let mut conn = cache.connection();
        let cache_key = format!("{}{}:{}", SESSION_KEY_PREFIX, self.session_id, key);

        let json_str = serde_json::to_string(value).map_err(|e| {
            crate::errors::ApiError::internal(format!("Session serialize error: {}", e))
        })?;

        let _: () = conn
            .set_ex(&cache_key, json_str, DEFAULT_SESSION_TTL.as_secs() as u64)
            .await
            .map_err(|e| crate::errors::ApiError::internal(format!("Redis set error: {}", e)))?;

        Ok(())
    }

    #[cfg(not(feature = "worker"))]
    pub async fn set<T: Serialize>(
        &self,
        _key: &str,
        _value: &T,
    ) -> Result<(), crate::errors::ApiError> {
        tracing::warn!("SessionStore requires the `worker` feature for Redis");
        Ok(())
    }

    /// Delete a specific key from the session.
    #[cfg(feature = "worker")]
    pub async fn remove(&self, key: &str) -> Result<(), crate::errors::ApiError> {
        let Some(cache) = self.cache else {
            return Ok(());
        };
        let mut conn = cache.connection();
        let cache_key = format!("{}{}:{}", SESSION_KEY_PREFIX, self.session_id, key);
        let _: () = conn
            .del(&cache_key)
            .await
            .map_err(|e| crate::errors::ApiError::internal(format!("Redis del error: {}", e)))?;
        Ok(())
    }

    #[cfg(not(feature = "worker"))]
    pub async fn remove(&self, _key: &str) -> Result<(), crate::errors::ApiError> {
        Ok(())
    }
}
