use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use drone::{control_server::ControlServer, DroneResult};
use tokio::time::sleep;
use tracing::{debug, error, info};

#[tokio::main(flavor = "current_thread")]
async fn main() -> DroneResult {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let control_server = ControlServer::new("./config.json")?;

    ctrlc::set_handler(move || {
        debug!("Ctrl+C detected!");
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let ctrlc_task = tokio::spawn(async move {
        while running.load(Ordering::SeqCst) {
            sleep(Duration::from_secs(1)).await;
        }
    });

    let control_server_task = tokio::spawn(async move {
        if let Err(e) = control_server.start().await {
            error!("Error starting control server: {e}");
        }
    });

    tokio::select! {
        _ = ctrlc_task => {
            info!("Ctrl-C handler task completed.");
        }
        _ = control_server_task => {
            info!("Control server task completed.");
        }
    };

    Ok(())
}
