use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use drone::app::App;
use drone::DroneResult;

fn main() -> DroneResult {
    // Load configuration
    let mut app = App::new("./config.json", Arc::new(AtomicBool::new(true)))?;

    // Read data
    app.fc_comms().listen()?;

    Ok(())
}
