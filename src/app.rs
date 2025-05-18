//! The main application
use tracing::info;

use crate::{app_data::DroneAppData, fc_comms::FcComms, logger::init_logger, DroneResult};

/// Applicaiton
pub struct App {
    /// Application data
    app_data: DroneAppData,
    /// Flight Controller Comms
    fc_comms: FcComms,
}

impl App {
    /// Create a new instance of the application
    pub fn new(app_data_file_path: &str) -> DroneResult<Self> {
        // Load configuration
        let app_data = DroneAppData::load_from_file(app_data_file_path);

        init_logger(&app_data.log_level().clone().into())?;
        info!("Starting Drone application...");

        let fc_comms = FcComms::new(&app_data)?;
        Ok(Self { app_data, fc_comms })
    }

    /// Get the application data
    pub fn app_data(&self) -> &DroneAppData {
        &self.app_data
    }

    /// Get the application data mutably
    pub fn fc_comms(&mut self) -> &mut FcComms {
        &mut self.fc_comms
    }
}
