//! App Data needed for the drone application

use serde::{Deserialize, Serialize};

use crate::logger::LogLevel;

/// Drone App Data
#[derive(Debug, Serialize, Deserialize)]
pub struct DroneAppData {
    /// App log level
    log_level: LogLevel,

    /// Port name for serial connection to the Flight Controller (FC)
    ///
    /// Default "/dev/ttyS0"
    fc_port_name: String,

    /// FC connection baud rate
    ///
    /// Default 420_000 baud rate
    fc_baud_rate: u32,
}
