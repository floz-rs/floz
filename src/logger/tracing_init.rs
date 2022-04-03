//! Tracing initialization with daily log rotation.
//!
//! Extracted from logger/logic/log.rs.

use std::sync::Once;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static INIT: Once = Once::new();

/// Initialize tracing with stdout + daily rotating file output.
///
/// Safe to call multiple times — only initializes once.
///
/// Log files are written to `./logs/<binary_name>.log` with daily rotation.
pub fn init_tracing() {
    INIT.call_once(|| {
        let log_dir = "logs";

        let log_file = match std::env::current_exe() {
            Ok(exe_path) => {
                exe_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| {
                        name.split('.')
                            .next()
                            .unwrap_or(name)
                            .to_string()
                    })
                    .unwrap_or_else(|| "application".to_string())
            }
            Err(_) => "application".to_string(),
        };

        let log_file = format!("{log_file}.log");

        let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, log_file);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        // Leak the guard so it lives for the program's duration
        Box::leak(Box::new(_guard));

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| {
                EnvFilter::new("debug,reqwest=trace,postgresql=off,postgresql::incoming::frame=off,postgresql::outgoing::frame=off")
            });

        if let Err(e) = tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::io::stdout))
            .with(fmt::layer().with_writer(non_blocking))
            .with(filter)
            .try_init()
        {
            eprintln!("Failed to initialize tracing: {e}");
            return;
        }

        tracing::info!("floz tracing initialized (daily log rotation)");
    });
}
