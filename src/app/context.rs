//! Application context — shared state across the application.
//!
//! This is the floz equivalent of Django's settings + connection pool.

use crate::config::Config;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Shared application context passed to every handler.
///
/// This replaces the monolithic `AppState` from api-rs with a
/// framework-level context that apps can extend.
#[derive(Clone)]
pub struct AppContext {
    /// Database connection pool (PostgreSQL)
    #[cfg(feature = "postgres")]
    pub db_pool: crate::db::PgDbPool,
    /// Database connection pool (SQLite)
    #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
    pub db_pool: crate::db::SqliteDbPool,
    /// Application configuration
    pub config: Config,
    /// Custom application state extensions
    pub extensions: Arc<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    /// Redis cache connection
    #[cfg(feature = "worker")]
    pub cache: Option<crate::cache::Cache>,
}

impl AppContext {
    /// Create a new AppContext with the given pool and config.
    pub fn new(
        #[cfg(feature = "postgres")] db_pool: crate::db::PgDbPool,
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))] db_pool: crate::db::SqliteDbPool,
        config: Config,
        extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    ) -> Self {
        Self {
            #[cfg(feature = "postgres")]
            db_pool,
            #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
            db_pool,
            config,
            extensions: Arc::new(extensions),
            #[cfg(feature = "worker")]
            cache: None,
        }
    }

    /// Get a database handle from the shared pool.
    ///
    /// This is the primary way to access the database in route handlers:
    /// ```ignore
    /// #[route(get: "/notes")]
    /// async fn list_notes() -> HttpResponse {
    ///     let notes = Note::all(&state.db()).await.unwrap();
    ///     HttpResponse::Ok().json(&notes)
    /// }
    /// ```
    #[cfg(feature = "postgres")]
    pub fn db(&self) -> floz_orm::Db {
        floz_orm::Db::from_pg_pool((*self.db_pool).clone())
    }

    #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
    pub fn db(&self) -> floz_orm::Db {
        floz_orm::Db::from_sqlite_pool((*self.db_pool).clone())
    }

    /// Get a Redis cache connection.
    /// Panics if the cache is not configured (e.g. missing REDIS_URL).
    #[cfg(feature = "worker")]
    pub fn cache(&self) -> &crate::cache::Cache {
        self.cache.as_ref().expect("Redis not configured. Set REDIS_URL env var.")
    }

    /// Enqueue a task message into the background worker system.
    #[cfg(feature = "worker")]
    pub async fn enqueue(&self, msg: crate::worker::TaskMessage) -> Result<(), crate::worker::TaskError> {
        let payload = serde_json::to_string(&msg)?;
        let key = format!("floz:queue:{}", msg.queue);
        let mut conn = self.cache().connection();
        redis::AsyncCommands::lpush::<_, _, ()>(&mut conn, key, payload).await?;
        Ok(())
    }

    /// Retrieve a reference to a custom injected state.
    /// Panics if the state is not available.
    pub fn ext<T: 'static>(&self) -> &T {
        self.try_ext::<T>().unwrap_or_else(|| {
            panic!(
                "Requested state extension {} was not injected via App::with()",
                std::any::type_name::<T>()
            )
        })
    }

    /// Try to retrieve a reference to a custom injected state.
    pub fn try_ext<T: 'static>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// Initialize AppContext from environment with auto-detected pool sizing
    /// and injected custom state extensions.
    pub async fn init(extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>) -> Self {
        let config = Config::from_env();
        
        #[cfg(any(feature = "postgres", feature = "sqlite"))]
        let worker_count = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
            
        #[cfg(feature = "postgres")]
        let db_pool = crate::db::pg_pool(worker_count).await;
        
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        let db_pool = crate::db::sqlite_pool().await;

        #[cfg(feature = "worker")]
        let cache = crate::cache::Cache::from_env().await.expect("Failed to initialize Redis cache");

        Self {
            #[cfg(feature = "postgres")]
            db_pool,
            #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
            db_pool,
            config,
            extensions: Arc::new(extensions),
            #[cfg(feature = "worker")]
            cache,
        }
    }
}
