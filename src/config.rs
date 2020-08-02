//! Configuration management for floz.
//!
//! Loads configuration from environment variables using dotenvy.
//! Follows Django/Rails convention — environment drives everything.

use once_cell::sync::Lazy;
use std::env;

/// Global config singleton — initialized once on first access.
static CONFIG: Lazy<Config> = Lazy::new(Config::from_env);

/// Application configuration loaded from environment variables.
///
/// # Required Environment Variables
/// - `DATABASE_URL` — PostgreSQL connection string
/// - `HOST` — Server bind host
/// - `PORT` — Server bind port
///
/// # Optional Environment Variables
/// - `REDIS_URL` — Redis connection string (for workers/cache)
/// - `JWT_TOKEN` — JWT signing secret (for auth)
/// - `JWT_AUDIENCE` — JWT audience claim
/// - `JWT_ISSUER` — JWT issuer claim
/// - `SERVER_ENV` — DEV / STAGING / PROD
/// - `ECHO` — Enable debug echo logging
#[derive(Debug, Clone)]
pub struct Config {
    // Database
    pub database_url: String,

    // Server
    pub host: String,
    pub port: String,
    pub server_env: String,

    // Redis (optional)
    pub redis_url: Option<String>,

    // Auth (optional)
    pub jwt_secret: Option<String>,
    pub jwt_audience: Option<String>,
    pub jwt_issuer: Option<String>,

    // Debug
    pub echo: bool,
}

impl Config {
    /// Load configuration from environment variables.
    /// Panics if required variables are missing.
    pub fn from_env() -> Self {
        // Load .env file if present
        dotenvy::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT").unwrap_or_else(|_| "3030".to_string()),
            server_env: env::var("SERVER_ENV").unwrap_or_else(|_| "DEV".to_string()),
            redis_url: env::var("REDIS_URL").ok(),
            jwt_secret: env::var("JWT_TOKEN").ok(),
            jwt_audience: env::var("JWT_AUDIENCE").ok(),
            jwt_issuer: env::var("JWT_ISSUER").ok(),
            echo: env::var("ECHO").is_ok(),
        }
    }

    /// Get the global config instance.
    pub fn global() -> &'static Config {
        &CONFIG
    }

    /// Check if running in development mode.
    pub fn is_dev(&self) -> bool {
        self.server_env == "DEV"
    }

    /// Check if running in production mode.
    pub fn is_prod(&self) -> bool {
        self.server_env == "PROD"
    }

    /// Get a custom environment variable with an optional default.
    pub fn get(key: &str) -> Option<String> {
        env::var(key).ok()
    }

    /// Get a custom environment variable, panicking if missing.
    pub fn require(key: &str) -> String {
        env::var(key).unwrap_or_else(|_| panic!("{key} environment variable must be set"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
