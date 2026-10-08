//! Telemetry received from the flight controller (FC)
use serde::{Deserialize, Serialize};

use super::crsf::{frame_type, CrsfFrame};
use crate::fc_comms::RcControls;

/// Everything we know about the drone. Sections are `None` until the FC reports them.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Telemetry {
    /// Battery / power
    pub battery: Option<Battery>,
    /// Attitude in degrees
    pub attitude: Option<Attitude>,
    /// Betaflight flight mode
    pub flight_mode: Option<FlightModeInfo>,
    /// GPS
    pub gps: Option<Gps>,
    /// Barometric altitude and vertical speed
    pub altitude: Option<Altitude>,
    /// RC channels currently being sent to the FC
    pub rc_sent: Option<RcControls>,
    /// Time (ms since Unix epoch) of the last valid frame from the FC
    pub last_fc_frame_ms: Option<u64>,
    /// Number of valid frames received from the FC
    pub frames_received: u64,
    /// Number of frames with a bad CRC
    pub crc_errors: u64,
}

/// Battery / power
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Battery {
    /// Voltage in volts
    pub voltage_v: f32,
    /// Current in amps
    pub current_a: f32,
    /// Power in watts (voltage × current)
    pub power_w: f32,
    /// Capacity used in mAh
    pub used_mah: u32,
    /// Remaining battery percentage
    pub remaining_pct: u8,
}

/// Attitude in degrees
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Attitude {
    /// Pitch in degrees
    pub pitch_deg: f32,
    /// Roll in degrees
    pub roll_deg: f32,
    /// Yaw in degrees
    pub yaw_deg: f32,
}

/// Betaflight flight mode
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FlightModeInfo {
    /// Mode name, e.g. "ACRO", "ANGL", "HOR"
    pub name: String,
    /// Whether the FC is armed (Betaflight appends `*` to the mode while disarmed)
    pub armed: bool,
}

/// GPS
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Gps {
    /// Latitude in degrees
    pub latitude: f64,
    /// Longitude in degrees
    pub longitude: f64,
    /// Ground speed in m/s
    pub ground_speed_ms: f32,
    /// Heading in degrees
    pub heading_deg: f32,
    /// GPS altitude in meters
    pub altitude_m: i32,
    /// Number of satellites
    pub satellites: u8,
}

/// Barometric altitude and vertical speed
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Altitude {
    /// Barometric altitude in meters
    pub baro_altitude_m: Option<f32>,
    /// Vertical speed in m/s
    pub vertical_speed_ms: Option<f32>,
}

impl Telemetry {
    /// Updates the telemetry from a CRSF frame. Returns `true` if the frame was understood.
    pub fn apply_frame(&mut self, frame: &CrsfFrame, now_ms: u64) -> bool {
        let p = frame.payload.as_slice();
        let handled = match frame.frame_type {
            frame_type::BATTERY_SENSOR if p.len() >= 8 => {
                let voltage_v = be_u16(p, 0) as f32 / 10.;
                let current_a = be_u16(p, 2) as f32 / 10.;
                self.battery = Some(Battery {
                    voltage_v,
                    current_a,
                    power_w: voltage_v * current_a,
                    used_mah: (p[4] as u32) << 16 | (p[5] as u32) << 8 | p[6] as u32,
                    remaining_pct: p[7],
                });
                true
            }
            frame_type::ATTITUDE if p.len() >= 6 => {
                let rad_to_deg = |v: i16| (v as f32 / 10_000.).to_degrees();
                self.attitude = Some(Attitude {
                    pitch_deg: rad_to_deg(be_i16(p, 0)),
                    roll_deg: rad_to_deg(be_i16(p, 2)),
                    yaw_deg: rad_to_deg(be_i16(p, 4)),
                });
                true
            }
            frame_type::FLIGHT_MODE => {
                let end = p.iter().position(|&b| b == 0).unwrap_or(p.len());
                let raw = String::from_utf8_lossy(&p[..end]).to_string();
                let armed = !raw.ends_with('*');
                self.flight_mode = Some(FlightModeInfo {
                    name: raw.trim_end_matches('*').to_string(),
                    armed,
                });
                true
            }
            frame_type::GPS if p.len() >= 15 => {
                self.gps = Some(Gps {
                    latitude: be_i32(p, 0) as f64 / 1e7,
                    longitude: be_i32(p, 4) as f64 / 1e7,
                    // km/h × 10 -> m/s
                    ground_speed_ms: be_u16(p, 8) as f32 / 36.,
                    heading_deg: be_u16(p, 10) as f32 / 100.,
                    altitude_m: be_u16(p, 12) as i32 - 1000,
                    satellites: p[14],
                });
                true
            }
            frame_type::VARIO if p.len() >= 2 => {
                // cm/s
                self.altitude
                    .get_or_insert_with(Altitude::default)
                    .vertical_speed_ms = Some(be_i16(p, 0) as f32 / 100.);
                true
            }
            frame_type::BARO_ALTITUDE if p.len() >= 2 => {
                let altitude = self.altitude.get_or_insert_with(Altitude::default);
                altitude.baro_altitude_m = Some(unpack_altitude(be_u16(p, 0)));
                if p.len() >= 3 {
                    altitude.vertical_speed_ms = Some(unpack_vertical_speed(p[2] as i8));
                }
                true
            }
            _ => false,
        };

        if handled {
            self.last_fc_frame_ms = Some(now_ms);
            self.frames_received += 1;
        }
        handled
    }
}

/// Unpacks CRSF baro altitude: MSB clear -> decimeters offset by 10000, MSB set -> meters
fn unpack_altitude(packed: u16) -> f32 {
    if packed & 0x8000 == 0 {
        (packed as f32 - 10_000.) / 10.
    } else {
        (packed & 0x7FFF) as f32
    }
}

/// Unpacks the log-scaled CRSF vertical speed into m/s
fn unpack_vertical_speed(packed: i8) -> f32 {
    const KL: f32 = 100.;
    const KR: f32 = 0.026;
    let cm_s = ((packed.unsigned_abs() as f32 * KR).exp() - 1.) * KL;
    packed.signum() as f32 * cm_s / 100.
}

pub(super) fn be_u16(p: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([p[i], p[i + 1]])
}

pub(super) fn be_i16(p: &[u8], i: usize) -> i16 {
    i16::from_be_bytes([p[i], p[i + 1]])
}

fn be_i32(p: &[u8], i: usize) -> i32 {
    i32::from_be_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(frame_type: u8, payload: Vec<u8>) -> CrsfFrame {
        CrsfFrame {
            frame_type,
            payload,
        }
    }

    #[test]
    fn decodes_battery_and_power() {
        let mut t = Telemetry::default();
        // 16.8 V, 12.5 A, 1234 mAh, 77 %
        let mut p = vec![];
        p.extend_from_slice(&168u16.to_be_bytes());
        p.extend_from_slice(&125u16.to_be_bytes());
        p.extend_from_slice(&[0x00, 0x04, 0xD2, 77]);
        assert!(t.apply_frame(&frame(frame_type::BATTERY_SENSOR, p), 42));

        let b = t.battery.unwrap();
        assert!((b.voltage_v - 16.8).abs() < 1e-4);
        assert!((b.current_a - 12.5).abs() < 1e-4);
        assert!((b.power_w - 210.).abs() < 1e-2);
        assert_eq!(b.used_mah, 1234);
        assert_eq!(b.remaining_pct, 77);
        assert_eq!(t.last_fc_frame_ms, Some(42));
    }

    #[test]
    fn decodes_attitude_in_degrees() {
        let mut t = Telemetry::default();
        let mut p = vec![];
        for rad in [0.5f32, -0.25, 3.0] {
            p.extend_from_slice(&((rad * 10_000.) as i16).to_be_bytes());
        }
        t.apply_frame(&frame(frame_type::ATTITUDE, p), 0);
        let a = t.attitude.unwrap();
        assert!((a.pitch_deg - 28.6479).abs() < 1e-2);
        assert!((a.roll_deg + 14.3239).abs() < 1e-2);
        assert!((a.yaw_deg - 171.887).abs() < 1e-2);
    }

    #[test]
    fn decodes_flight_mode_and_armed_flag() {
        let mut t = Telemetry::default();
        t.apply_frame(&frame(frame_type::FLIGHT_MODE, b"ANGL*\0".to_vec()), 0);
        assert_eq!(
            t.flight_mode,
            Some(FlightModeInfo {
                name: "ANGL".into(),
                armed: false
            })
        );
        t.apply_frame(&frame(frame_type::FLIGHT_MODE, b"ACRO\0".to_vec()), 0);
        assert!(t.flight_mode.unwrap().armed);
    }

    #[test]
    fn decodes_gps() {
        let mut t = Telemetry::default();
        let mut p = vec![];
        p.extend_from_slice(&475_000_000i32.to_be_bytes());
        p.extend_from_slice(&(-1_220_000_000i32).to_be_bytes());
        p.extend_from_slice(&360u16.to_be_bytes()); // 36 km/h = 10 m/s
        p.extend_from_slice(&9_000u16.to_be_bytes()); // 90°
        p.extend_from_slice(&1_120u16.to_be_bytes()); // 120 m
        p.push(11);
        t.apply_frame(&frame(frame_type::GPS, p), 0);
        let g = t.gps.unwrap();
        assert!((g.latitude - 47.5).abs() < 1e-9);
        assert!((g.longitude + 122.).abs() < 1e-9);
        assert!((g.ground_speed_ms - 10.).abs() < 1e-4);
        assert!((g.heading_deg - 90.).abs() < 1e-4);
        assert_eq!(g.altitude_m, 120);
        assert_eq!(g.satellites, 11);
    }

    #[test]
    fn decodes_vario_and_baro_altitude() {
        let mut t = Telemetry::default();
        t.apply_frame(&frame(frame_type::VARIO, (-150i16).to_be_bytes().to_vec()), 0);
        assert_eq!(t.altitude.as_ref().unwrap().vertical_speed_ms, Some(-1.5));

        // 12.3 m in decimeters + 10000 offset, vertical speed packed 0
        let mut p = (10_123u16).to_be_bytes().to_vec();
        p.push(0);
        t.apply_frame(&frame(frame_type::BARO_ALTITUDE, p), 0);
        let alt = t.altitude.unwrap();
        assert!((alt.baro_altitude_m.unwrap() - 12.3).abs() < 1e-3);
        assert_eq!(alt.vertical_speed_ms, Some(0.));
    }

    #[test]
    fn ignores_short_and_unknown_frames() {
        let mut t = Telemetry::default();
        assert!(!t.apply_frame(&frame(frame_type::BATTERY_SENSOR, vec![1, 2]), 0));
        assert!(!t.apply_frame(&frame(0x55, vec![1, 2, 3]), 0));
        assert_eq!(t.frames_received, 0);
    }
}
