use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use drone::{app::App, DroneResult};

use tracing::{debug, info};

fn main() -> DroneResult {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    App::new("./config.json", running.clone())?;

    ctrlc::set_handler(move || {
        debug!("Ctrl+C detected!");
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        sleep(Duration::from_secs(1));
    }

    info!("Exiting gracefully.");

    Ok(())
}
