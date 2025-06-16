//! The main error type for the cloud-comms crate.

/// Drone related errors
#[derive(Debug, thiserror::Error)]
pub enum DroneError {
    /// Logger Errors
    #[error(transparent)]
    SetGlobalDefaultError(#[from] tracing::subscriber::SetGlobalDefaultError),

    /// Serial port errors
    #[cfg(feature = "serialport")]
    #[error(transparent)]
    SerialPortError(#[from] serialport::Error),

    /// Serde Errors
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    /// UDP server errors
    #[error(transparent)]
    UdpError(#[from] std::io::Error),

    /// Arc Mutex errors
    #[error("Generic error: {0}")]
    ArcMutexError(String),

    #[cfg(feature = "dualsense")]
    /// DualSense controller errors
    #[error(transparent)]
    DualSenseError(#[from] gilrs::Error),
}
