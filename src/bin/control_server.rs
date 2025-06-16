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
    // Create failure flag to handle Ctrl+C gracefully
    let failure_flag_ctrlc_task = Arc::new(AtomicBool::new(false));
    let failure_flag_checker = failure_flag_ctrlc_task.clone();

    let ctrlc_task = tokio::spawn(async move {
        ctrlc::set_handler(move || {
            debug!("Ctrl+C detected!");
            failure_flag_ctrlc_task.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        loop {
            if failure_flag_checker.load(Ordering::SeqCst) {
                info!("Ctrl-C handler detected failure flag, exiting...");
                break;
            }

            sleep(Duration::from_secs(1)).await;
        }
    });

    let control_server_task = {
        let control_server = ControlServer::new("./config.json")?;
        tokio::spawn(async move {
            if let Err(e) = control_server.start().await {
                error!("Error starting control server: {e}");
            }
        })
    };

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
