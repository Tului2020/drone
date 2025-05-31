//! Library file for Drone application
#![warn(missing_docs)]
pub mod app;
pub mod app_data;
#[cfg(any(feature = "control_server", feature = "dualsense"))]
pub mod control_server;
#[cfg(feature = "dualsense")]
pub mod dualsense_controller;
pub mod error;
pub mod fc_comms;
pub mod logger;
pub mod messages;
#[cfg(feature = "udp_server")]
pub mod udp_server;

/// A type alias for the result of a Conductor operation.
pub type DroneResult<T = ()> = std::result::Result<T, error::DroneError>;

/// Gets time in milliseconds since the Unix epoch.
pub fn get_time_ms() -> u128 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}
