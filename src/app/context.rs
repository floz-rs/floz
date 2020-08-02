//! Application context — shared state across the application.
//!
//! This is the floz equivalent of Django's settings + connection pool.

use crate::config::Config;
use crate::db::DbPool;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Shared application context passed to every handler.
///
/// This replaces the monolithic `AppState` from api-rs with a
/// framework-level context that apps can extend.
#[derive(Clone)]
pub struct AppContext {
    /// Database connection pool
    pub db_pool: DbPool,
    /// Application configuration
    pub config: Config,
    /// Custom application state extensions
    pub extensions: Arc<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl AppContext {
    /// Create a new AppContext with the given pool and config.
    pub fn new(db_pool: DbPool, config: Config, extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>) -> Self {
        Self { 
            db_pool, 
            config, 
            extensions: Arc::new(extensions), 
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
    pub fn db(&self) -> floz_orm::Db {
        floz_orm::Db::from_pool((*self.db_pool).clone())
    }

    /// Retrieve a reference to a custom injected state.
    /// Panics if the state is not available.
    pub fn ext<T: 'static>(&self) -> &T {
        self.try_ext::<T>().unwrap_or_else(|| {
            panic!("Requested state extension {} was not injected via App::with()", std::any::type_name::<T>())
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
        let worker_count = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        let db_pool = crate::db::pool(worker_count).await;

        Self { 
            db_pool, 
            config, 
            extensions: Arc::new(extensions), 
        }
    }
}
