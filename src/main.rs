use drone::app_data::DroneAppData;
use drone::logger::init_logger;
use drone::DroneResult;
use tracing::info;

fn main() -> DroneResult {
    let app_data = DroneAppData::load_from_file("./config.json");

    init_logger(&app_data.log_level().clone().into())?;

    info!("Starting Drone application...");

    // let port_name = "/dev/ttyS0";
    // let baud_rate = 420_000;

    // let mut port = serialport::new(port_name, baud_rate)
    //     .timeout(Duration::from_millis(1000))
    //     .open()?;

    // // Read data
    // let mut buffer = [0u8; 1024];
    // match port.read(&mut buffer) {
    //     Ok(n) => println!("Received: {:?}", &buffer[..n]),
    //     Err(e) => eprintln!("Read failed: {}", e),
    // }

    Ok(())
}
