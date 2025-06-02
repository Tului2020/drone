//! DualSense controller module
mod state;

use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
    time::Duration,
};

use actix_web::web;
use hidapi::HidApi;
use state::{DualSenseControllerState, FlightMode};
use tokio::time::sleep;
use tracing::{error, info};

use crate::{control_server::UdpClient, fc_comms::RcControls, get_time_ms, DroneResult};

/// Threshold for smoother function to avoid sudden jumps in the controller's response
const SMOOTH_THRESHOLD: i8 = 10;
/// Threshold for button press time to determine if a button is pressed or not
const BUTTON_PRESS_TIME_THRESHOLD_MS: u128 = 100;
/// Range for RC control sliders
const RC_CONTROL_SLIDER_RANGE: f32 = 128.;

/// Represents a DualSense controller with its fields and methods.
#[allow(dead_code)]
#[derive(Clone)]
pub struct DualsenseController {
    // rust list out PS5 controller fields
    lx: i8,
    ly: i8,
    rx: i8,
    ry: i8,
    l2_val: u8, // analog 0-255
    r2_val: u8, // analog 0-255
    up: bool,
    right: bool,
    down: bool,
    left: bool,
    square: bool,
    cross: bool,
    circle: bool,
    triangle: bool,
    l1: bool,
    r1: bool,
    l2: bool, // digital click
    r2: bool,
    create: bool,
    options: bool,
    l3: bool,
    r3: bool,
    ps: bool,
    touch_btn: bool,
    mic_mute: bool,
    dualsense_state: DualSenseControllerState,
}

impl DualsenseController {
    /// Creates a new instance of `DualsenseController` with default values and creates a thread to read input from the controller.
    ///
    /// # Returns
    ///
    /// A new `DualsenseController` instance with all fields initialized to their default values.
    pub fn new(udp_client: web::Data<UdpClient>) -> Arc<Mutex<Self>> {
        let dualsense_controller = Arc::new(Mutex::new(DualsenseController::default()));
        let dualsense_controller_clone = dualsense_controller.clone();

        std::thread::spawn(move || {
            let tokio_runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            tokio_runtime.block_on(async {
                if let Err(e) =
                    DualsenseController::connect(dualsense_controller_clone, udp_client).await
                {
                    error!("Error connecting to DualSense controller: {e}");
                } else {
                    info!("DualSense controller connected successfully.");
                }
            });
        });

        dualsense_controller
    }

    /// Connects to the DualSense controller and starts reading input.
    pub async fn connect(
        dualsense_controller: Arc<Mutex<DualsenseController>>,
        udp_client: web::Data<UdpClient>,
    ) -> DroneResult<()> {
        let api = HidApi::new()?;
        let pad = api.open(0x054c, 0x0ce6)?; // Sony DualSense (USB)
        let mut buf = [0u8; 64];
        let mut previous_rc_controls = RcControls::default();
        // Keep track of the time when each button was last pressed
        // TODO: make button_last_pressed_tracker a field of DualsenseController
        let mut button_last_pressed_tracker = HashMap::new();
        // Exponential backoff for sending RC controls
        let mut backoff_multiplier = 1;

        loop {
            let n = pad.read_timeout(&mut buf, 1000)?;
            if n == 0 || buf[0] != 0x01 {
                continue;
            } // empty / other reports

            let now_ms = get_time_ms();

            /* ---------------- sticks ---------------- */
            let lx = (buf[1] as i16 - 128) as i8;
            let ly = (buf[2] as i16 - 128) as i8;
            let rx = (buf[3] as i16 - 128) as i8;
            let ry = (buf[4] as i16 - 128) as i8;

            /* ---------------- triggers -------------- */
            let l2_val = buf[5]; // analog 0-255
            let r2_val = buf[6];

            /* ---------------- hats & face ----------- */
            let hats_face = buf[8];
            let dpad_nib = hats_face & 0x0F;
            let face_nib = (hats_face >> 4) & 0x0F;

            let up = matches!(dpad_nib, 0 | 1 | 7);
            let right = matches!(dpad_nib, 1 | 2 | 3);
            let down = matches!(dpad_nib, 3 | 4 | 5);
            let left = matches!(dpad_nib, 5 | 6 | 7);

            let square = face_nib & 0b0001 != 0;
            let cross = face_nib & 0b0010 != 0;
            let circle = face_nib & 0b0100 != 0;
            let triangle = face_nib & 0b1000 != 0;

            /* ---------------- shoulders & misc ------ */
            let btns1 = buf[9];
            let btns2 = buf[10];

            let l1 = btns1 & 0x01 != 0;
            let r1 = btns1 & 0x02 != 0;
            let l2 = btns1 & 0x04 != 0; // digital click
            let r2 = btns1 & 0x08 != 0;
            let create = btns1 & 0x10 != 0;
            let options = btns1 & 0x20 != 0;
            let l3 = btns1 & 0x40 != 0;
            let r3 = btns1 & 0x80 != 0;

            let ps = btns2 & 0x01 != 0;
            let touch_btn = btns2 & 0x02 != 0;
            let mic_mute = btns2 & 0x04 != 0;

            dualsense_controller.lock().unwrap().update(
                lx, ly, rx, ry, l2_val, r2_val, up, right, down, left, square, cross, circle,
                triangle, l1, r1, l2, // digital click
                r2, create, options, l3, r3, ps, touch_btn, mic_mute,
            );

            let new_rc_controls = {
                let mut dualsense_controller = dualsense_controller.lock().unwrap();
                let dualsense_controller = dualsense_controller.sample(); // get a snapshot of the current state

                println!("\n{dualsense_controller}"); // print the controller state
                dualsense_controller.to_rc_controls(
                    &previous_rc_controls,
                    &mut button_last_pressed_tracker,
                    now_ms,
                )
            };
            println!("{new_rc_controls}\n"); // print the RC controls

            if let Err(e) = udp_client.send_rc(new_rc_controls).await {
                error!("Failed to send RC controls: {e}");
                // Implement exponential backoff
                backoff_multiplier = (backoff_multiplier * 2).min(8); // Cap backoff to a maximum value
            } else {
                // Reset backoff if sending was successful
                backoff_multiplier = 1;
            }

            previous_rc_controls.update(&new_rc_controls);

            sleep(Duration::from_millis(10 * backoff_multiplier)).await; // avoid busy loop
        }
    }

    /// Updates the controller state with new values.
    pub fn update(
        &mut self,
        lx: i8,
        ly: i8,
        rx: i8,
        ry: i8,
        l2_val: u8,
        r2_val: u8,
        up: bool,
        right: bool,
        down: bool,
        left: bool,
        square: bool,
        cross: bool,
        circle: bool,
        triangle: bool,
        l1: bool,
        r1: bool,
        l2: bool, // digital click
        r2: bool,
        create: bool,
        options: bool,
        l3: bool,
        r3: bool,
        ps: bool,
        touch_btn: bool,
        mic_mute: bool,
    ) {
        self.lx = lx;
        self.ly = ly;
        self.rx = rx;
        self.ry = ry;
        self.l2_val = l2_val;
        self.r2_val = r2_val;
        self.up = up;
        self.right = right;
        self.down = down;
        self.left = left;
        self.square = square;
        self.cross = cross;
        self.circle = circle;
        self.triangle = triangle;
        self.l1 = l1;
        self.r1 = r1;
        self.l2 = l2;
        self.r2 = r2;
        self.create = create;
        self.options = options;
        self.l3 = l3;
        self.r3 = r3;
        self.ps = ps;
        self.touch_btn = touch_btn;
        self.mic_mute = mic_mute;
    }

    /// Returns the current state of the controller
    pub fn sample(&mut self) -> &mut Self {
        self
    }

    /// Converts the controller state to `RcControls` for communication with the flight controller.
    pub fn to_rc_controls(
        &mut self,
        previous_rc_controls: &RcControls,
        button_last_pressed_tracker: &mut HashMap<String, u128>,
        now_ms: u128,
    ) -> RcControls {
        // Ratio to convert DualSense values to RC controls
        let dualsense_to_rc = 500. / RC_CONTROL_SLIDER_RANGE;

        let roll = (1500i16 + (Self::smoother(self.rx) * dualsense_to_rc) as i16) as u16;
        let pitch = (1500i16 + (-Self::smoother(self.ry) * dualsense_to_rc) as i16) as u16;
        let yaw = (1500i16 + (Self::smoother(self.lx) * dualsense_to_rc) as i16) as u16;

        let base_thr = self.dualsense_state.flight_mode().get_base_thr();
        let thr_multiplier = (2000 - base_thr) as f32 / RC_CONTROL_SLIDER_RANGE;
        let thr = (base_thr + (-Self::smoother(self.ly) * thr_multiplier) as i16) as u16;

        let aux1 = Self::dualsense_to_fc(
            self.ps,
            "ps",
            button_last_pressed_tracker,
            now_ms,
            vec![1000, 1700, 1900],
            previous_rc_controls.aux1,
        );

        let aux2 = Self::dualsense_to_fc(
            self.r1,
            "r1",
            button_last_pressed_tracker,
            now_ms,
            vec![1000, 1400, 1900],
            previous_rc_controls.aux2,
        );

        // Update Flight Mode based on create button press
        if self.options {
            let is_new_press = Self::is_new_press(button_last_pressed_tracker, "options", now_ms);
            if is_new_press {
                match self.dualsense_state.flight_mode() {
                    FlightMode::Ready | FlightMode::Land => {
                        self.dualsense_state.set_flight_mode(FlightMode::Hover);
                    }
                    FlightMode::Hover => {
                        self.dualsense_state.set_flight_mode(FlightMode::Land);
                    }
                }
            }
        }
        if self.create {
            let is_new_press = Self::is_new_press(button_last_pressed_tracker, "create", now_ms);
            if is_new_press {
                match self.dualsense_state.flight_mode() {
                    FlightMode::Ready | FlightMode::Hover => {
                        self.dualsense_state.set_flight_mode(FlightMode::Land);
                    }
                    FlightMode::Land => {
                        self.dualsense_state.set_flight_mode(FlightMode::Ready);
                    }
                }
            }
        }

        RcControls {
            roll,
            pitch,
            yaw,
            thr,
            aux1,
            aux2,
            aux3: 1000,
            aux4: 1000,
        }
    }

    // Converts DualSense button presses to flight controller values.
    fn dualsense_to_fc(
        dualsense_button: bool,
        dualsense_button_name: &str,
        button_last_pressed_tracker: &mut HashMap<String, u128>,
        now_ms: u128,
        // NOTE: first value is the default
        ordered_fc_values: Vec<u16>,
        previous_fc_value: u16,
    ) -> u16 {
        if dualsense_button {
            // Check if new press of button
            let is_new_press =
                Self::is_new_press(button_last_pressed_tracker, dualsense_button_name, now_ms);

            if is_new_press {
                let is_index_found = ordered_fc_values
                    .iter()
                    .position(|&x| x == previous_fc_value);

                if let Some(found_index) = is_index_found {
                    return ordered_fc_values[(found_index + 1) % ordered_fc_values.len()];
                }
            }
        }
        previous_fc_value
    }

    fn is_new_press(
        button_last_pressed_tracker: &mut HashMap<String, u128>,
        button_name: &str,
        now_ms: u128,
    ) -> bool {
        let mut new_press_bool = false;
        if let Some(last_time_pressed) = button_last_pressed_tracker.get(button_name) {
            if now_ms - *last_time_pressed > BUTTON_PRESS_TIME_THRESHOLD_MS {
                new_press_bool = true; // New press detected
            }
        } else {
            new_press_bool = true; // First press detected
        }

        // Update the last pressed time
        button_last_pressed_tracker.insert(button_name.to_string(), now_ms);
        new_press_bool
    }

    /// Smooths the input values to avoid sudden jumps in the controller's response.
    fn smoother(x: i8) -> f32 {
        // NOTE: need to check as i16 to avoid overflow
        if (x as i16).abs() < SMOOTH_THRESHOLD as i16 {
            return 0.; // If the value is below the threshold, return 0
        }

        (x as f32).powi(3) / 128.0_f32.powi(2)
    }
}

impl Default for DualsenseController {
    fn default() -> Self {
        DualsenseController {
            lx: 0,
            ly: 0,
            rx: 0,
            ry: 0,
            l2_val: 0,
            r2_val: 0,
            up: false,
            right: false,
            down: false,
            left: false,
            square: false,
            cross: false,
            circle: false,
            triangle: false,
            l1: false,
            r1: false,
            l2: false, // digital click
            r2: false,
            create: false,
            options: false,
            l3: false,
            r3: false,
            ps: false,
            touch_btn: false,
            mic_mute: false,
            dualsense_state: DualSenseControllerState::default(),
        }
    }
}

impl Display for DualsenseController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LS({:>4},{:>4}) RS({:>4},{:>4})  DPad ↑{} ↓{} ←{} →{}  □{} ×{} ○{} △{}  \
             L1{} R1{} L2{:3}/{} R2{:3}/{}  L3{} R3{}  CRT{} OPT{} PS{} TP{} Mute{}",
            self.lx,
            self.ly,
            self.rx,
            self.ry,
            if self.up { "X" } else { " " },
            if self.down { "X" } else { " " },
            if self.left { "X" } else { " " },
            if self.right { "X" } else { " " },
            if self.square { "X" } else { " " },
            if self.cross { "X" } else { " " },
            if self.circle { "X" } else { " " },
            if self.triangle { "X" } else { " " },
            if self.l1 { "X" } else { " " },
            if self.r1 { "X" } else { " " },
            self.l2_val,
            if self.l2 { "X" } else { " " },
            self.r2_val,
            if self.r2 { "X" } else { " " },
            if self.l3 { "X" } else { " " },
            if self.r3 { "X" } else { " " },
            if self.create { "X" } else { " " },
            if self.options { "X" } else { " " },
            if self.ps { "X" } else { " " },
            if self.touch_btn { "X" } else { " " },
            if self.mic_mute { "X" } else { " " },
        )
    }
}
