//! Simulated FC telemetry for development without a flight controller.
//!
//! The fake drone loosely follows the RC commands it receives, so the UI and controller
//! tuning can be exercised end to end on a laptop.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::sleep,
    time::{Duration, Instant},
};

use super::{
    msp::{FcStatus, Imu},
    telemetry::{Altitude, Attitude, Battery, FlightModeInfo, Gps},
    RcControls, Telemetry,
};
use crate::get_time_ms;

const TICK: Duration = Duration::from_millis(50);
/// Full stick deflection maps to this many degrees of tilt
const MAX_TILT_DEG: f32 = 35.;
/// Full yaw stick turns this many degrees per second
const MAX_YAW_RATE_DPS: f32 = 180.;

/// Runs the simulator until `running` is cleared
pub fn run(
    rc_controls: Arc<Mutex<RcControls>>,
    telemetry: Arc<Mutex<Telemetry>>,
    running: Arc<AtomicBool>,
) {
    let start = Instant::now();
    let mut yaw_deg = 0f32;
    let mut used_mah = 0f32;
    let mut altitude_m = 0f32;

    while running.load(Ordering::SeqCst) {
        let rc = *rc_controls.lock().unwrap();
        let t = start.elapsed().as_secs_f32();
        let dt = TICK.as_secs_f32();
        let stick = |us: u16| ((us as f32 - 1500.) / 500.).clamp(-1., 1.);
        let throttle = ((rc.thr as f32 - 1000.) / 1000.).clamp(0., 1.);
        let armed = rc.aux1 >= 1800;

        yaw_deg = (yaw_deg + stick(rc.yaw) * MAX_YAW_RATE_DPS * dt).rem_euclid(360.);
        let current_a = if armed { 1.5 + 40. * throttle.powi(2) } else { 0.4 };
        used_mah += current_a * 1000. * dt / 3600.;
        let voltage_v = (16.8 - used_mah / 1500. * 2.6 - current_a * 0.015).max(13.2);
        let vertical_speed_ms = if armed { (throttle - 0.41) * 8. } else { 0. };
        altitude_m = (altitude_m + vertical_speed_ms * dt).max(0.);
        let pitch_deg = stick(rc.pitch) * MAX_TILT_DEG + (t * 1.3).sin() * 0.8;
        let roll_deg = stick(rc.roll) * MAX_TILT_DEG + (t * 1.7).cos() * 0.8;
        let motor = |mix: f32| {
            if armed {
                (1000. + 1000. * (throttle + mix * 0.15).clamp(0.05, 1.)) as u16
            } else {
                0
            }
        };

        {
            let mut tm = telemetry.lock().unwrap();
            tm.battery = Some(Battery {
                voltage_v,
                current_a,
                power_w: voltage_v * current_a,
                used_mah: used_mah as u32,
                remaining_pct: (100. - used_mah / 15.).clamp(0., 100.) as u8,
            });
            tm.attitude = Some(Attitude {
                pitch_deg,
                roll_deg,
                yaw_deg,
            });
            tm.flight_mode = Some(FlightModeInfo {
                name: match rc.aux2 {
                    ..=1200 => "ACRO",
                    1201..=1600 => "ANGL",
                    _ => "HOR",
                }
                .into(),
                armed,
            });
            tm.gps = Some(Gps {
                latitude: 47.6062 + (yaw_deg as f64).to_radians().cos() * 1e-5,
                longitude: -122.3321 + (yaw_deg as f64).to_radians().sin() * 1e-5,
                ground_speed_ms: (stick(rc.pitch).powi(2) + stick(rc.roll).powi(2)).sqrt() * 12.,
                heading_deg: yaw_deg,
                altitude_m: altitude_m as i32,
                satellites: 12,
            });
            tm.altitude = Some(Altitude {
                baro_altitude_m: Some(altitude_m),
                vertical_speed_ms: Some(vertical_speed_ms),
            });
            let (p, r, y) = (stick(rc.pitch), stick(rc.roll), stick(rc.yaw));
            tm.motors = Some(vec![
                motor(-p - r + y),
                motor(-p + r - y),
                motor(p - r - y),
                motor(p + r + y),
            ]);
            tm.fc_status = Some(FcStatus {
                cycle_time_us: 125,
                cpu_load_pct: 18 + (t.sin() * 3.) as u16,
                sensors: vec!["ACC".into(), "BARO".into(), "GPS".into(), "GYRO".into()],
                arming_disable_flags: if armed {
                    vec![]
                } else if rc.thr > 1050 {
                    vec!["THROTTLE".into()]
                } else {
                    vec!["ARM_SWITCH".into()]
                },
                reboot_required: false,
                cpu_temp_c: Some(42),
            });
            tm.imu = Some(Imu {
                acc_raw: [
                    (roll_deg.to_radians().sin() * 2048.) as i16,
                    (pitch_deg.to_radians().sin() * 2048.) as i16,
                    2048,
                ],
                gyro_dps: [
                    (r * 200.) as i16,
                    (p * 200.) as i16,
                    (y * MAX_YAW_RATE_DPS) as i16,
                ],
                mag_raw: [0, 0, 0],
            });
            tm.last_fc_frame_ms = Some(get_time_ms() as u64);
            tm.frames_received += 1;
        }

        sleep(TICK);
    }
}
