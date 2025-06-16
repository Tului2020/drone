//! DualSense controller module
mod state;

use core::f32;
use std::{fmt::Display, time::Duration};

use actix_web::web;
use gilrs::{Axis::*, Button::*, Event, EventType, Gilrs};
use state::{DualSenseControllerState, FlightMode};
use tokio::time::sleep;
use tracing::error;

use crate::{control_server::UdpClient, fc_comms::RcControls, DroneResult};

/// Threshold for smoother function to avoid sudden jumps in the controller's response
const SMOOTH_THRESHOLD: f32 = 10.;
/// Range for RC control sliders
const RC_CONTROL_SLIDER_RANGE: f32 = 128.;
/// Linear smoothing factor for controller input values
const LINEAR_SMOOTHING_FACTOR: f32 =
    RC_CONTROL_SLIDER_RANGE / (RC_CONTROL_SLIDER_RANGE - SMOOTH_THRESHOLD);

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
    pub async fn new(udp_client: web::Data<UdpClient>) -> DroneResult {
        let mut controller_api = Gilrs::new()?;
        let mut dualsense_controller = DualsenseController::default();
        let mut previous_rc_controls = RcControls::default();
        // Exponential backoff for sending RC controls
        let mut backoff_multiplier = 1;

        loop {
            if let Some(Event { event, .. }) = controller_api.next_event() {
                let mut is_new_event = true;

                match event {
                    EventType::ButtonPressed(button, _) => {
                        match button {
                            South => dualsense_controller.cross = true,
                            East => dualsense_controller.circle = true,
                            North => dualsense_controller.triangle = true,
                            West => dualsense_controller.square = true,
                            LeftTrigger => dualsense_controller.l1 = true,
                            LeftTrigger2 => dualsense_controller.l2 = true,
                            RightTrigger => dualsense_controller.r1 = true,
                            RightTrigger2 => dualsense_controller.r2 = true,
                            Select => dualsense_controller.create = true,
                            Start => dualsense_controller.options = true,
                            LeftThumb => dualsense_controller.l3 = true,
                            RightThumb => dualsense_controller.r3 = true,
                            DPadUp => dualsense_controller.up = true,
                            DPadDown => dualsense_controller.down = true,
                            DPadLeft => dualsense_controller.left = true,
                            DPadRight => dualsense_controller.right = true,
                            // C => todo!(),
                            // Z => todo!(),
                            // Mode => todo!(),
                            _ => is_new_event = false,
                        }
                    }
                    EventType::ButtonReleased(button, _) => {
                        is_new_event = false;
                        match button {
                            South => dualsense_controller.cross = false,
                            East => dualsense_controller.circle = false,
                            North => dualsense_controller.triangle = false,
                            West => dualsense_controller.square = false,
                            LeftTrigger => dualsense_controller.l1 = false,
                            LeftTrigger2 => dualsense_controller.l2 = false,
                            RightTrigger => dualsense_controller.r1 = false,
                            RightTrigger2 => dualsense_controller.r2 = false,
                            Select => dualsense_controller.create = false,
                            Start => dualsense_controller.options = false,
                            LeftThumb => dualsense_controller.l3 = false,
                            RightThumb => dualsense_controller.r3 = false,
                            DPadUp => dualsense_controller.up = false,
                            DPadDown => dualsense_controller.down = false,
                            DPadLeft => dualsense_controller.left = false,
                            DPadRight => dualsense_controller.right = false,
                            // C => todo!(),
                            // Z => todo!(),
                            // Mode => todo!(),
                            _ => is_new_event = false,
                        }
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        match axis {
                            LeftStickX => dualsense_controller.lx = (value * 128.0) as i8,
                            LeftStickY => dualsense_controller.ly = -(value * 128.0) as i8,
                            RightStickX => dualsense_controller.rx = (value * 128.0) as i8,
                            RightStickY => dualsense_controller.ry = -(value * 128.0) as i8,
                            // RightZ => println!("Right Z: {}", value),
                            // DPadX => println!("DPad X: {}", value),
                            // DPadY => println!("DPad Y: {}", value),
                            // LeftZ => println!("Left Z: {}", value),
                            _ => is_new_event = false,
                        }
                    }
                    _ => is_new_event = false,
                }

                if is_new_event {
                    let rc_controls = dualsense_controller
                        .to_rc_controls(&previous_rc_controls, &Smoother::Cubic);

                    if let Err(e) = udp_client.send_rc(rc_controls.clone()).await {
                        error!("Failed to send RC controls: {e}");
                        // Implement exponential backoff
                        backoff_multiplier = (backoff_multiplier * 2).min(8); // Cap backoff to a maximum value
                    } else {
                        // Reset backoff if sending was successful
                        backoff_multiplier = 1;
                    }

                    previous_rc_controls.update(&rc_controls);
                }
            }
            sleep(Duration::from_millis(1 * backoff_multiplier)).await;
        }
    }

    /// Converts the controller state to `RcControls` for communication with the flight controller.
    pub fn to_rc_controls(
        &mut self,
        previous_rc_controls: &RcControls,
        smoother: &Smoother,
    ) -> RcControls {
        // Ratio to convert DualSense values to RC controls
        let dualsense_to_rc = 500. / RC_CONTROL_SLIDER_RANGE;

        let roll = (1500i16 + (Self::smoother(self.rx, smoother) * dualsense_to_rc) as i16) as u16;
        let pitch =
            (1500i16 + (-Self::smoother(self.ry, smoother) * dualsense_to_rc) as i16) as u16;
        let yaw = (1500i16 + (Self::smoother(self.lx, smoother) * dualsense_to_rc) as i16) as u16;

        let base_thr = self.dualsense_state.flight_mode().get_base_thr();
        let thr_multiplier = (2000 - base_thr) as f32 / RC_CONTROL_SLIDER_RANGE;
        let thr = (base_thr + (-Self::smoother(self.ly, smoother) * thr_multiplier) as i16) as u16;

        // ------------------------------------------------- FC Controls -------------------------------------------------
        let mut aux1 = Self::dualsense_to_fc(
            self.l1,
            vec![
                1000, // Disarm
                1700, // Pre-arm
                1900, // Arm
            ],
            previous_rc_controls.aux1,
        );

        if self.r2 {
            aux1 = 1000; // R2 is killswitch
        }

        let aux2 = Self::dualsense_to_fc(
            self.r1,
            vec![
                1000, // Acro mode
                1400, // Angle mode
                1900, // Horizon mode
            ],
            previous_rc_controls.aux2,
        );

        // -------------------------------------------------- Custom Controls -------------------------------------------------
        // Update Flight Mode based on create button press
        if self.options {
            match self.dualsense_state.flight_mode() {
                FlightMode::Ready | FlightMode::Land | FlightMode::Custom(_) => {
                    self.dualsense_state.set_flight_mode(FlightMode::Hover);
                }
                FlightMode::Hover => {
                    self.dualsense_state.set_flight_mode(FlightMode::Land);
                }
            }
        }
        if self.create {
            match self.dualsense_state.flight_mode() {
                FlightMode::Ready | FlightMode::Hover | FlightMode::Custom(_) => {
                    self.dualsense_state.set_flight_mode(FlightMode::Land);
                }
                FlightMode::Land => {
                    self.dualsense_state.set_flight_mode(FlightMode::Ready);
                }
            }
        }
        if self.up {
            let current_default_thr = self.dualsense_state.flight_mode().get_base_thr();

            // Custom flight mode
            self.dualsense_state
                .set_flight_mode(FlightMode::Custom(current_default_thr + 10));
        }
        if self.down {
            let current_default_thr = self.dualsense_state.flight_mode().get_base_thr();

            // Custom flight mode
            self.dualsense_state
                .set_flight_mode(FlightMode::Custom(current_default_thr - 10));
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
        // NOTE: first value is the default
        ordered_fc_values: Vec<u16>,
        previous_fc_value: u16,
    ) -> u16 {
        if dualsense_button {
            let is_index_found = ordered_fc_values
                .iter()
                .position(|&x| x == previous_fc_value);

            if let Some(found_index) = is_index_found {
                return ordered_fc_values[(found_index + 1) % ordered_fc_values.len()];
            }
        }
        previous_fc_value
    }

    /// Smooths the input values to avoid sudden jumps in the controller's response.
    fn smoother(x: i8, smoother: &Smoother) -> f32 {
        // NOTE: need to check as i16 to avoid overflow
        if (x as i16).abs() < SMOOTH_THRESHOLD as i16 {
            return 0.; // If the value is below the threshold, return 0
        }

        match smoother {
            Smoother::Sinusoidal => {
                let x_abs = x.abs() as f32;
                let sign = if x < 0 { -1. } else { 1. };

                sign * RC_CONTROL_SLIDER_RANGE
                    * (x_abs * f32::consts::PI / (2. * RC_CONTROL_SLIDER_RANGE)).sin()
            }
            Smoother::Linear => {
                let x_abs = x.abs() as f32;
                let sign = if x < 0 { -1. } else { 1. };
                sign * (x_abs - SMOOTH_THRESHOLD) * LINEAR_SMOOTHING_FACTOR
            }
            Smoother::Cubic => (x as f32).powi(3) / RC_CONTROL_SLIDER_RANGE.powi(2),
        }
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

/// Enum representing different smoothing methods for controller input values.
pub enum Smoother {
    /// Sinusoidal smoothing
    Sinusoidal,
    /// Linear smoothing
    Linear,
    /// Cubic smoothing
    Cubic,
}
