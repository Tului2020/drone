//! Contains the logger for the cloud-comms application.
use crate::DroneResult;
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
