use std::{thread::sleep, time::Duration};

use drone::app::App;
use drone::DroneResult;
use tracing::{error, info};

fn main() -> DroneResult {
    // Load configuration
    let mut app = App::new("./config.json")?;

    // Read data
    let mut buffer = [0u8; 1024];
    match app.fc_comms().port().read(&mut buffer) {
        Ok(n) => info!("Received: {:?}", &buffer[..n]),
        Err(e) => error!("Read failed: {e}"),
    }

    let buffer: [u8; 26] = [
        0xc8, 0x18, 0x16, 0xdf, 0xfb, 0xde, 0x01, 0xbe, 0xf7, 0xdb, 0x5f, 0xfc, 0xf6, 0x17, 0xbf,
        0xf8, 0xc5, 0x2f, 0x7e, 0xf1, 0x8b, 0x5f, 0xfc, 0xe2, 0x17, 0x5e,
    ];

    for _i in 0..10 {
        app.fc_comms().port().write_all(&buffer).unwrap();

        sleep(Duration::from_millis(20));
    }

    Ok(())
}
