//! Module for FC communications
use std::time::Duration;

use serialport::SerialPort;
use tracing::{debug, error};

use crate::{app_data::DroneAppData, error::DroneError, DroneResult};

/// FC communications
pub struct FcComms {
    /// Serial port
    port: Box<dyn SerialPort + 'static>,
}

impl FcComms {
    /// Create a new instance of the FC communications
    pub fn new(app_data: &DroneAppData) -> DroneResult<Self> {
        {
            let port_name = app_data.fc_port_name();
            let baud_rate = app_data.fc_baud_rate();
            let port = serialport::new(port_name, baud_rate)
                .timeout(Duration::from_millis(1000))
                .open()
                .map_err(|e| {
                    error!("{e}");
                    DroneError::SerialPort("Could not open port".to_string())
                })?;
            debug!("Serial port opened: {port_name} at {baud_rate} baud");
            Ok(Self { port })
        }
    }

    /// Get the serial port
    pub fn port(&mut self) -> &mut dyn SerialPort {
        self.port.as_mut()
    }
}
