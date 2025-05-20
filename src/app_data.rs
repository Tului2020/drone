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
    /// Control server port
    control_server_port: u16,
    /// UDP server address
    udp_server_addr: String,
}

impl DroneAppData {
    /// Creates a new instance of `DroneAppData` with default values.
    ///
    /// # Returns
    ///
    /// A `DroneAppData` instance with default values.
    pub fn new(
        log_level: LogLevel,
        fc_port_name: String,
        fc_baud_rate: u32,
        control_server_port: u16,
        udp_server_addr: String,
    ) -> Self {
        Self {
            log_level,
            fc_port_name,
            fc_baud_rate,
            control_server_port,
            udp_server_addr,
        }
    }

    /// Returns the log level for the application.
    pub fn log_level(&self) -> &LogLevel {
        &self.log_level
    }

    /// Returns the port name for the serial connection to the Flight Controller (FC).
    pub fn fc_port_name(&self) -> &str {
        &self.fc_port_name
    }

    /// Returns the baud rate for the serial connection to the Flight Controller (FC).
    pub fn fc_baud_rate(&self) -> u32 {
        self.fc_baud_rate
    }

    /// Returns the control server port.
    pub fn control_server_port(&self) -> u16 {
        self.control_server_port
    }

    /// Returns the UDP server address.
    pub fn udp_server_addr(&self) -> &str {
        &self.udp_server_addr
    }

    /// Loads the configuration from a JSON file.
    pub fn load_from_file(file_path: &str) -> Self {
        let file = std::fs::File::open(file_path).expect("Unable to open config file");
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).expect("Unable to parse config file")
    }
}

impl Default for DroneAppData {
    fn default() -> Self {
        Self {
            log_level: LogLevel::TRACE,
            fc_port_name: "/dev/ttyS0".to_string(),
            fc_baud_rate: 420_000,
            control_server_port: 8080,
            udp_server_addr: "0.0.0.0:8080".to_string(),
        }
    }
}
