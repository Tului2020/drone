//! This module contains a way to make sure that DualSense controller is connected
use std::sync::{Arc, Mutex};

use actix_web::web;
use drone::{control_server::UdpClient, dualsense_controller::DualSenseController};
use tracing::{error, info};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // init_logger(&Level::DEBUG).expect("Failed to initialize logger");
    let dual_sense_controller = Arc::new(Mutex::new(DualSenseController::default()));
    let dual_sense_controller_clone = dual_sense_controller.clone();

    let udp_client: web::Data<UdpClient> =
        web::Data::new(UdpClient::new("Test".to_string()).await.unwrap());

    if let Err(e) = DualSenseController::connect(dual_sense_controller_clone, udp_client).await {
        error!("Error connecting to DualSense controller: {e}");
    } else {
        info!("DualSense controller connected successfully.");
    }
}
