//! Library file for Drone application
#![warn(missing_docs)]
pub mod app;
pub mod app_data;
pub mod error;
pub mod fc_comms;
pub mod logger;

/// A type alias for the result of a Conductor operation.
pub type DroneResult<T = ()> = std::result::Result<T, error::DroneError>;
