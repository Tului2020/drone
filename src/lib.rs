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

/// How often the control server sends a heartbeat to the drone, in milliseconds.
///
/// Hardcoded (not configurable) so the control server and the drone always agree on it.
/// The drone resets the RC controls after 3 intervals without a heartbeat.
pub const HEARTBEAT_INTERVAL_MS: u64 = 100;

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

/// Default path of the application configuration file.
pub const DEFAULT_CONFIG_PATH: &str = "./config.json";

/// Returns the config file path: the first CLI argument if given, otherwise [`DEFAULT_CONFIG_PATH`].
pub fn config_path() -> String {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string())
}
