//! Library file for Drone application
#![warn(missing_docs)]
pub mod app;
pub mod app_data;
#[cfg(feature = "control_server")]
pub mod control_server;
pub mod error;
pub mod fc_comms;
pub mod logger;
#[cfg(any(feature = "control_server", feature = "udp_server"))]
pub mod messages;
#[cfg(feature = "udp_server")]
pub mod udp_server;

/// A type alias for the result of a Conductor operation.
pub type DroneResult<T = ()> = std::result::Result<T, error::DroneError>;
