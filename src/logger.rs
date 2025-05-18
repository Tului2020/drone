//! Contains the logger for the cloud-comms application.
use crate::DroneResult;
use serde::{Deserialize, Serialize};
use tracing_subscriber::FmtSubscriber;

/// Initializes the logger for the Conductor application.
pub fn init_logger() -> DroneResult<()> {
    let subscriber: FmtSubscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    // Set the subscriber as the global default
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/// Application log level
#[derive(Debug, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace level
    TRACE,
    /// Debug level
    DEBUG,
    /// Info level
    INFO,
    /// Warn level
    WARN,
    /// Error level
    ERROR,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::TRACE => Self::TRACE,
            LogLevel::DEBUG => Self::DEBUG,
            LogLevel::INFO => Self::INFO,
            LogLevel::WARN => Self::WARN,
            LogLevel::ERROR => Self::ERROR,
        }
    }
}

impl From<tracing::Level> for LogLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Self::TRACE,
            tracing::Level::DEBUG => Self::DEBUG,
            tracing::Level::INFO => Self::INFO,
            tracing::Level::WARN => Self::WARN,
            tracing::Level::ERROR => Self::ERROR,
        }
    }
}
