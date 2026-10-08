//! Raspberry Pi entrypoint: receives RC commands over UDP and drives the flight controller.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use drone::{app::App, config_path, DroneResult};

use tokio::time::sleep;
use tracing::debug;

#[tokio::main(flavor = "current_thread")]
async fn main() -> DroneResult {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    App::new(&config_path(), running.clone())?;

    ctrlc::set_handler(move || {
        debug!("Ctrl+C detected!");
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
