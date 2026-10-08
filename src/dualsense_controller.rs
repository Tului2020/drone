//! DualSense controller module
pub mod settings;
mod state;

use std::{
    fmt::Display,
    path::PathBuf,
    sync::RwLock,
    time::{Duration, Instant},
};

use actix_web::web;
use gilrs::{Axis::*, Button, Button::*, Event, EventType, Gilrs};
use serde::Serialize;
use settings::{ControllerSettings, SlewLimiter};
use state::{DualSenseControllerState, FlightMode};
use tokio::{sync::watch, time::sleep};
use tracing::{error, info};

use crate::{control_server::UdpClient, fc_comms::RcControls, get_time_ms, DroneResult};

/// How often the stick outputs are recomputed and sent
const TICK: Duration = Duration::from_millis(20);
/// Full roll/pitch/yaw deflection in µs around center
const STICK_RANGE_US: f32 = 500.;
/// Lowest throttle value ever sent
const MIN_THROTTLE_US: f32 = 885.;
/// Highest throttle value ever sent
const MAX_THROTTLE_US: f32 = 2000.;
/// AUX1 values cycled by L1: disarm, pre-arm, arm
const AUX1_ARM_CYCLE: [u16; 3] = [1000, 1700, 1900];
/// AUX2 values cycled by R1: acro, angle, horizon
const AUX2_MODE_CYCLE: [u16; 3] = [1000, 1400, 1900];
/// Base throttle change per D-pad press
const BASE_THROTTLE_STEP: i16 = 10;

/// Shared controller state: tunable settings and a live snapshot for the UI
pub struct ControllerHub {
    settings: RwLock<ControllerSettings>,
    settings_path: PathBuf,
    snapshot: watch::Sender<ControllerSnapshot>,
}

impl ControllerHub {
    /// Loads settings from `settings_path` (or defaults if it does not exist)
    pub fn new(settings_path: PathBuf) -> Self {
        Self {
            settings: RwLock::new(ControllerSettings::load(&settings_path)),
            settings_path,
            snapshot: watch::channel(ControllerSnapshot::default()).0,
        }
    }

    /// Current settings
    pub fn settings(&self) -> ControllerSettings {
        *self.settings.read().unwrap()
    }

    /// Validates, applies and saves new settings
    pub fn update_settings(&self, new_settings: ControllerSettings) -> Result<(), String> {
        new_settings.validate()?;
        *self.settings.write().unwrap() = new_settings;
        new_settings
            .save(&self.settings_path)
            .map_err(|e| format!("settings applied but not saved: {e}"))?;
        info!("Controller settings updated: {new_settings:?}");
        Ok(())
    }

    /// Subscribes to live controller snapshots
    pub fn subscribe(&self) -> watch::Receiver<ControllerSnapshot> {
        self.snapshot.subscribe()
    }
}

/// What the controller is doing right now, for the UI
#[derive(Debug, Clone, Default, Serialize)]
pub struct ControllerSnapshot {
    /// Whether a gamepad is connected
    pub connected: bool,
    /// Raw stick positions in [-1, 1]
    pub sticks: Sticks,
    /// RC values after curves and slew limiting
    pub rc: Option<RcControls>,
    /// Ground-station flight mode (Ready / Hover / Land / Custom)
    pub flight_mode: String,
    /// Base throttle of the flight mode
    pub base_throttle: i16,
    /// When this snapshot was taken (ms since Unix epoch)
    pub updated_ms: u64,
}

/// Raw stick positions in [-1, 1]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Sticks {
    /// Right stick X
    pub roll: f32,
    /// Right stick Y (up = positive)
    pub pitch: f32,
    /// Left stick X
    pub yaw: f32,
    /// Left stick Y (up = positive)
    pub throttle: f32,
}

/// Represents a DualSense controller with its fields and methods.
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct DualsenseController {
    sticks: Sticks,
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
    l2: bool,
    r2: bool,
    create: bool,
    options: bool,
    l3: bool,
    r3: bool,
    aux1: u16,
    aux2: u16,
    dualsense_state: DualSenseControllerState,
}

/// One slew limiter per stick channel
#[derive(Default)]
struct OutputSlew {
    roll: SlewLimiter,
    pitch: SlewLimiter,
    yaw: SlewLimiter,
    thr: SlewLimiter,
}

impl DualsenseController {
    /// Reads the controller forever, sending RC controls to the drone every [`TICK`] when they change.
    pub async fn new(
        udp_client: web::Data<UdpClient>,
        hub: web::Data<ControllerHub>,
    ) -> DroneResult {
        let mut controller_api = Gilrs::new()?;
        let mut controller = DualsenseController {
            aux1: AUX1_ARM_CYCLE[0],
            aux2: AUX2_MODE_CYCLE[0],
            ..Default::default()
        };
        let mut slew = OutputSlew::default();
        let mut last_sent: Option<RcControls> = None;
        let mut last_tick = Instant::now();
        // Exponential backoff for sending RC controls
        let mut backoff_multiplier = 1;

        loop {
            while let Some(Event { event, .. }) = controller_api.next_event() {
                controller.handle_event(event);
            }

            let dt = last_tick.elapsed();
            if dt >= TICK * backoff_multiplier {
                last_tick = Instant::now();
                let settings = hub.settings();
                let rc_controls =
                    controller.to_rc_controls(&settings, &mut slew, dt.as_secs_f32().min(0.1));

                if last_sent != Some(rc_controls) {
                    match udp_client.send_rc(rc_controls).await {
                        Ok(_) => {
                            backoff_multiplier = 1;
                            last_sent = Some(rc_controls);
                        }
                        Err(e) => {
                            error!("Failed to send RC controls: {e}");
                            backoff_multiplier = (backoff_multiplier * 2).min(8);
                        }
                    }
                }

                hub.snapshot.send_replace(ControllerSnapshot {
                    connected: controller_api.gamepads().any(|(_, g)| g.is_connected()),
                    sticks: controller.sticks,
                    rc: Some(rc_controls),
                    flight_mode: controller.dualsense_state.flight_mode().to_string(),
                    base_throttle: controller.dualsense_state.flight_mode().get_base_thr(),
                    updated_ms: get_time_ms() as u64,
                });
            }

            sleep(Duration::from_millis(1)).await;
        }
    }

    /// Updates button/stick state and runs press actions
    fn handle_event(&mut self, event: EventType) {
        match event {
            EventType::ButtonPressed(button, _) => {
                self.set_button(button, true);
                self.on_press(button);
            }
            EventType::ButtonReleased(button, _) => self.set_button(button, false),
            EventType::AxisChanged(axis, value, _) => match axis {
                LeftStickX => self.sticks.yaw = value,
                LeftStickY => self.sticks.throttle = value,
                RightStickX => self.sticks.roll = value,
                RightStickY => self.sticks.pitch = value,
                _ => {}
            },
            _ => {}
        }
    }

    fn set_button(&mut self, button: Button, pressed: bool) {
        match button {
            South => self.cross = pressed,
            East => self.circle = pressed,
            North => self.triangle = pressed,
            West => self.square = pressed,
            LeftTrigger => self.l1 = pressed,
            LeftTrigger2 => self.l2 = pressed,
            RightTrigger => self.r1 = pressed,
            RightTrigger2 => self.r2 = pressed,
            Select => self.create = pressed,
            Start => self.options = pressed,
            LeftThumb => self.l3 = pressed,
            RightThumb => self.r3 = pressed,
            DPadUp => self.up = pressed,
            DPadDown => self.down = pressed,
            DPadLeft => self.left = pressed,
            DPadRight => self.right = pressed,
            _ => {}
        }
    }

    /// Actions that happen once per button press (holding a button does not repeat them)
    fn on_press(&mut self, button: Button) {
        match button {
            // L1 cycles disarm -> pre-arm -> arm
            LeftTrigger => self.aux1 = Self::next_in_cycle(&AUX1_ARM_CYCLE, self.aux1),
            // R2 is the killswitch
            RightTrigger2 => self.aux1 = AUX1_ARM_CYCLE[0],
            // R1 cycles acro -> angle -> horizon
            RightTrigger => self.aux2 = Self::next_in_cycle(&AUX2_MODE_CYCLE, self.aux2),
            Start => {
                let next = match self.dualsense_state.flight_mode() {
                    FlightMode::Hover => FlightMode::Land,
                    _ => FlightMode::Hover,
                };
                self.dualsense_state.set_flight_mode(next);
            }
            Select => {
                let next = match self.dualsense_state.flight_mode() {
                    FlightMode::Land => FlightMode::Ready,
                    _ => FlightMode::Land,
                };
                self.dualsense_state.set_flight_mode(next);
            }
            DPadUp | DPadDown => {
                let step = if button == DPadUp {
                    BASE_THROTTLE_STEP
                } else {
                    -BASE_THROTTLE_STEP
                };
                let base = self.dualsense_state.flight_mode().get_base_thr();
                self.dualsense_state
                    .set_flight_mode(FlightMode::Custom(base + step));
            }
            _ => {}
        }
    }

    /// Converts the controller state to `RcControls` using the sensitivity settings.
    fn to_rc_controls(
        &self,
        settings: &ControllerSettings,
        slew: &mut OutputSlew,
        dt_s: f32,
    ) -> RcControls {
        let stick_us = |value: f32| 1500. + value * STICK_RANGE_US;
        let roll = stick_us(settings.roll.apply(self.sticks.roll));
        let pitch = stick_us(settings.pitch.apply(self.sticks.pitch));
        let yaw = stick_us(settings.yaw.apply(self.sticks.yaw));

        let base_thr = self.dualsense_state.flight_mode().get_base_thr() as f32;
        let thr = (base_thr
            + settings.throttle.apply(self.sticks.throttle) * (MAX_THROTTLE_US - base_thr))
            .clamp(MIN_THROTTLE_US, MAX_THROTTLE_US);

        let (roll, pitch, yaw, mut thr) = (
            slew.roll.step(roll, settings.roll.slew_us_per_s, dt_s),
            slew.pitch.step(pitch, settings.pitch.slew_us_per_s, dt_s),
            slew.yaw.step(yaw, settings.yaw.slew_us_per_s, dt_s),
            slew.thr.step(thr, settings.throttle.slew_us_per_s, dt_s),
        );

        // Holding R2 always kills: disarm and drop throttle immediately
        let aux1 = if self.r2 {
            thr = MIN_THROTTLE_US;
            slew.thr.reset(thr);
            AUX1_ARM_CYCLE[0]
        } else {
            self.aux1
        };

        RcControls {
            roll: roll.round() as u16,
            pitch: pitch.round() as u16,
            yaw: yaw.round() as u16,
            thr: thr.round() as u16,
            aux1,
            aux2: self.aux2,
            aux3: 1000,
            aux4: 1000,
        }
    }

    /// Returns the value after `current` in `cycle` (or `current` if it is not in the cycle)
    fn next_in_cycle(cycle: &[u16], current: u16) -> u16 {
        cycle
            .iter()
            .position(|&x| x == current)
            .map_or(current, |i| cycle[(i + 1) % cycle.len()])
    }
}

impl Display for DualsenseController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = |pressed: bool| if pressed { "X" } else { " " };
        write!(
            f,
            "LS({:>5.2},{:>5.2}) RS({:>5.2},{:>5.2})  DPad ↑{} ↓{} ←{} →{}  □{} ×{} ○{} △{}  \
             L1{} R1{} L2{} R2{}  L3{} R3{}  CRT{} OPT{}",
            self.sticks.yaw,
            self.sticks.throttle,
            self.sticks.roll,
            self.sticks.pitch,
            x(self.up),
            x(self.down),
            x(self.left),
            x(self.right),
            x(self.square),
            x(self.cross),
            x(self.circle),
            x(self.triangle),
            x(self.l1),
            x(self.r1),
            x(self.l2),
            x(self.r2),
            x(self.l3),
            x(self.r3),
            x(self.create),
            x(self.options),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::AxisSettings;

    fn controller() -> DualsenseController {
        DualsenseController {
            aux1: AUX1_ARM_CYCLE[0],
            aux2: AUX2_MODE_CYCLE[0],
            ..Default::default()
        }
    }

    /// Same as a gilrs ButtonPressed event (gilrs event codes cannot be constructed in tests)
    fn press(c: &mut DualsenseController, button: Button) {
        c.set_button(button, true);
        c.on_press(button);
    }

    fn release(c: &mut DualsenseController, button: Button) {
        c.set_button(button, false);
    }

    #[test]
    fn l1_cycles_arm_once_per_press() {
        let mut c = controller();
        press(&mut c, LeftTrigger);
        assert_eq!(c.aux1, 1700);
        // Holding and ticking does not cycle again
        let mut slew = OutputSlew::default();
        let settings = ControllerSettings::default();
        for _ in 0..5 {
            c.to_rc_controls(&settings, &mut slew, 0.02);
        }
        assert_eq!(c.aux1, 1700);
        release(&mut c, LeftTrigger);
        press(&mut c, LeftTrigger);
        assert_eq!(c.aux1, 1900);
        press(&mut c, LeftTrigger);
        assert_eq!(c.aux1, 1000);
    }

    #[test]
    fn r2_kills_while_held() {
        let mut c = controller();
        press(&mut c, LeftTrigger);
        press(&mut c, LeftTrigger);
        c.sticks.throttle = 1.;
        let mut slew = OutputSlew::default();
        let settings = ControllerSettings::default();
        press(&mut c, RightTrigger2);
        let rc = c.to_rc_controls(&settings, &mut slew, 0.02);
        assert_eq!((rc.aux1, rc.thr), (1000, 885));
    }

    #[test]
    fn rate_caps_full_forward_pitch() {
        let mut c = controller();
        c.sticks.pitch = 1.;
        let mut settings = ControllerSettings::default();
        settings.pitch = AxisSettings {
            rate: 0.4,
            expo: 0.,
            deadzone: 0.,
            slew_us_per_s: 0.,
        };
        let rc = c.to_rc_controls(&settings, &mut OutputSlew::default(), 0.02);
        assert_eq!(rc.pitch, 1700);
        assert_eq!(rc.roll, 1500);
    }

    #[test]
    fn slew_ramps_sudden_full_press() {
        let mut c = controller();
        let mut settings = ControllerSettings::default();
        settings.pitch.slew_us_per_s = 1000.;
        let mut slew = OutputSlew::default();
        c.to_rc_controls(&settings, &mut slew, 0.02);
        c.sticks.pitch = 1.;
        let rc = c.to_rc_controls(&settings, &mut slew, 0.02);
        assert_eq!(rc.pitch, 1520);
    }

    #[test]
    fn default_settings_match_original_cubic_endpoints() {
        let mut c = controller();
        c.sticks.roll = 1.;
        c.sticks.yaw = -1.;
        let rc = c.to_rc_controls(
            &ControllerSettings::default(),
            &mut OutputSlew::default(),
            0.02,
        );
        assert_eq!((rc.roll, rc.yaw, rc.thr), (2000, 1000, 1000));
    }

    #[test]
    fn dpad_adjusts_base_throttle() {
        let mut c = controller();
        press(&mut c, Start);
        assert_eq!(c.dualsense_state.flight_mode().get_base_thr(), 1410);
        press(&mut c, DPadUp);
        assert_eq!(c.dualsense_state.flight_mode().get_base_thr(), 1420);
    }
}
