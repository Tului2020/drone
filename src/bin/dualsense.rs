//! This module contains a way to make sure that DualSense controller is connected
use actix_web::web;
use drone::{
    app_data::DroneAppData, control_server::UdpClient, dualsense_controller::DualsenseController,
    logger::init_logger,
};
use tracing::{error, info, Level};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    init_logger(&Level::DEBUG).expect("Failed to initialize logger");

    // Load configuration
    let app_data = DroneAppData::load_from_file("./config.json");
    let udp_client = web::Data::new(
        UdpClient::new(app_data.udp_server_addr().to_string())
            .await
            .unwrap(),
    );

    if let Err(e) = DualsenseController::new(udp_client).await {
        error!("Error connecting to DualSense controller: {e}");
    } else {
        info!("DualSense controller connected successfully.");
    }
}
