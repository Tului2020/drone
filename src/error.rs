//! The main error type for the cloud-comms crate.

/// Drone related errors
#[derive(Debug, thiserror::Error)]
pub enum DroneError {
    /// Logger Errors
    #[error(transparent)]
    SetGlobalDefaultError(#[from] tracing::subscriber::SetGlobalDefaultError),
}
