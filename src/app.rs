//! The main application

use std::time::Duration;

use serialport::SerialPort;
use tracing::info;

use crate::{app_data::DroneAppData, logger::init_logger, DroneResult};

/// Applicaiton
pub struct App {
    /// Application data
    app_data: DroneAppData,
    /// Serial port
    port: Box<dyn SerialPort + 'static>,
}

impl App {
    /// Create a new instance of the application
    pub fn new(app_data_file_path: &str) -> DroneResult<Self> {
        // Load configuration
        let app_data = DroneAppData::load_from_file(app_data_file_path);

        init_logger(&app_data.log_level().clone().into())?;
        info!("Starting Drone application...");

        let port = serialport::new(app_data.fc_port_name(), app_data.fc_baud_rate())
            .timeout(Duration::from_millis(1000))
            .open()
            .unwrap();

        Ok(Self { app_data, port })
    }

    /// Get the application data
    pub fn app_data(&self) -> &DroneAppData {
        &self.app_data
    }

    /// Get the application data mutably
    pub fn port(&mut self) -> &mut dyn SerialPort {
        self.port.as_mut()
    }
}
