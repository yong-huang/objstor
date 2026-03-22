use crate::config::ServerConfig;
use crate::error::Result;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging(config: &ServerConfig) -> Result<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Create logs directory
    std::fs::create_dir_all(&config.log_dir)?;

    // File appender with daily rotation
    let file_appender =
        tracing_appender::rolling::daily(&config.log_dir, "objstor.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Console layer
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // File layer with JSON format
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .json()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logging initialized");
    Ok(guard)
}
