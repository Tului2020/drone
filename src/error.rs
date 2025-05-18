//! The main error type for the cloud-comms crate.

/// Drone related errors
#[derive(Debug, thiserror::Error)]
pub enum DroneError {
    /// Logger Errors
    #[error(transparent)]
    SetGlobalDefaultError(#[from] tracing::subscriber::SetGlobalDefaultError),

    /// Not found
    #[error("Serial Port: {0}")]
    SerialPort(String),
}
