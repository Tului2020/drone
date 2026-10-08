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
    /// Control server address and port
    control_server_address: String,
    /// UDP server address
    udp_server_addr: String,
    /// UDP port the control server listens on for telemetry from the drone
    #[serde(default = "default_telemetry_port")]
    telemetry_port: u16,
    /// How often the drone sends telemetry to the control server, in milliseconds
    #[serde(default = "default_telemetry_interval_ms")]
    telemetry_interval_ms: u64,
    /// The drone's H.264 camera stream (`host:port`). Defaults to the drone's host
    /// (from `udp_server_addr`) on port 2222, see `raspi_services/live_camera.service`.
    #[serde(default)]
    camera_stream_addr: Option<String>,
}

/// TCP port the drone's camera stream listens on
const DEFAULT_CAMERA_PORT: u16 = 2222;

fn default_telemetry_port() -> u16 {
    8081
}

fn default_telemetry_interval_ms() -> u64 {
    100
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
        control_server_address: String,
        udp_server_addr: String,
        telemetry_port: u16,
        telemetry_interval_ms: u64,
    ) -> Self {
        Self {
            log_level,
            fc_port_name,
            fc_baud_rate,
            control_server_address,
            udp_server_addr,
            telemetry_port,
            telemetry_interval_ms,
            camera_stream_addr: None,
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

    /// Returns the control server address and  port.
    pub fn control_server_address(&self) -> &String {
        &self.control_server_address
    }

    /// Returns the UDP server address.
    pub fn udp_server_addr(&self) -> &str {
        &self.udp_server_addr
    }

    /// Returns the UDP port the control server listens on for telemetry.
    pub fn telemetry_port(&self) -> u16 {
        self.telemetry_port
    }

    /// Returns how often the drone sends telemetry, in milliseconds.
    pub fn telemetry_interval_ms(&self) -> u64 {
        self.telemetry_interval_ms
    }

    /// Returns the drone's camera stream address (`host:port`).
    pub fn camera_stream_addr(&self) -> String {
        self.camera_stream_addr.clone().unwrap_or_else(|| {
            let host = self
                .udp_server_addr
                .rsplit_once(':')
                .map_or(self.udp_server_addr.as_str(), |(host, _)| host);
            format!("{host}:{DEFAULT_CAMERA_PORT}")
        })
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
            control_server_address: "127.0.0.1:8080".to_string(),
            udp_server_addr: "0.0.0.0:8080".to_string(),
            telemetry_port: default_telemetry_port(),
            telemetry_interval_ms: default_telemetry_interval_ms(),
            camera_stream_addr: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_stream_defaults_to_drone_host_on_port_2222() {
        let mut app_data = DroneAppData {
            udp_server_addr: "tului-hackathon.local:8080".into(),
            ..Default::default()
        };
        assert_eq!(app_data.camera_stream_addr(), "tului-hackathon.local:2222");

        app_data.camera_stream_addr = Some("10.0.0.5:9000".into());
        assert_eq!(app_data.camera_stream_addr(), "10.0.0.5:9000");
    }
}
