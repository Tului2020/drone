use std::thread::sleep;
use std::time::Duration;

use drone::app::App;
use drone::DroneResult;

fn main() -> DroneResult {
    // Load configuration
    let mut app = App::new("./config.json")?;

    let mut x: f32 = 0.0;

    loop {
        let val = ((250.0 * x.sin()) as i16 + 1500) as u16;
        println!("val: {}", val);

        app.fc_comms().set_rc_controls(
            Some(val),
            Some(val),
            None,
            Some(val),
            Some(val),
            Some(val),
            Some(val),
            Some(val),
        );

        x += 0.001;
        sleep(Duration::from_millis(30));
    }
}
