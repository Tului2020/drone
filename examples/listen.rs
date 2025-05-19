use drone::app::App;
use drone::DroneResult;

fn main() -> DroneResult {
    // Load configuration
    let mut app = App::new("./config.json")?;

    // Read data
    app.fc_comms().listen()?;

    Ok(())
}
