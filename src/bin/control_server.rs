#[cfg(feature = "control_server")]
use drone::{app_data::DroneAppData, control_server::ControlServer, logger::init_logger};
use tracing::error;
#[cfg(feature = "control_server")]
use tracing::info;

#[tokio::main]
async fn main() {
    #[cfg(feature = "control_server")]
    {
        // Load configuration
        let app_data = DroneAppData::load_from_file("./config.json");

        init_logger(&app_data.log_level().clone().into()).unwrap();
        info!("Starting control server...");
        if let Err(e) = ControlServer::new(&app_data).start().await {
            error!("Error starting control server: {e}");
        }
    }

    #[cfg(not(feature = "control_server"))]
    error!("Control server feature is not enabled. Please enable it in your Cargo.toml.");
}
