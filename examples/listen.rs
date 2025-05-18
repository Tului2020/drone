use drone::app::App;
use drone::DroneResult;
use tracing::{error, info};

fn main() -> DroneResult {
    // Load configuration
    let mut app = App::new("./config.json")?;

    // Read data
    let mut buffer = [0u8; 1024];
    match app.port().read(&mut buffer) {
        Ok(n) => info!("Received: {:?}", &buffer[..n]),
        Err(e) => error!("Read failed: {e}"),
    }

    Ok(())
}
