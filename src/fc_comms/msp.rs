//! MSP (MultiWii Serial Protocol) tunnelled over CRSF extended frames.
//!
//! Requests go out as `MSP_REQ` (0x7A) frames and Betaflight answers with one or more
//! `MSP_RESP` (0x7B) chunks in its telemetry slots. Each chunk is
//! `[DEST][ORIGIN][STATUS][...]` where `STATUS` = `error(7) | version(6..5) | start(4) | seq(3..0)`.
//! The first chunk of an MSPv1 message carries `[SIZE][CMD]` before the data. There is no MSP
//! checksum over CRSF; the CRSF CRC already covers each chunk.
//! Reference: betaflight `src/main/telemetry/msp_shared.c`.
use serde::{Deserialize, Serialize};

use super::crsf::{build_frame, frame_type, SYNC_BYTE};

/// Address used as origin for our requests (handset / radio)
const RADIO_ADDRESS: u8 = 0xEA;
const STATUS_SEQUENCE_MASK: u8 = 0x0F;
const STATUS_START_MASK: u8 = 0x10;
const STATUS_VERSION_MASK: u8 = 0x60;
const STATUS_VERSION_SHIFT: u8 = 5;
const STATUS_ERROR_MASK: u8 = 0x80;
const MSP_V1: u8 = 1;

/// MSP command ids
pub mod cmd {
    /// Reboot the flight controller (optional payload: reboot mode, 0 = firmware)
    pub const REBOOT: u8 = 68;
    /// Raw accelerometer, gyro (deg/s) and magnetometer
    pub const RAW_IMU: u8 = 102;
    /// `;`-separated names of the configured modes ("boxes"), in flight-mode-flag bit order
    pub const BOXNAMES: u8 = 116;
    /// Motor outputs
    pub const MOTOR: u8 = 104;
    /// Battery: cell count, voltage, mAh drawn, current, alert state
    pub const BATTERY_STATE: u8 = 130;
    /// Extended status: loop time, CPU load, arming-disable flags...
    pub const STATUS_EX: u8 = 150;
}

/// MSP_REBOOT mode: normal firmware reboot
pub const REBOOT_MODE_FIRMWARE: u8 = 0;

/// Betaflight battery alert states, indexed by `batteryState_e`
const BATTERY_STATE_NAMES: [&str; 5] = ["OK", "WARNING", "CRITICAL", "NOT_PRESENT", "INIT"];

/// Betaflight 4.5 arming-disable flag names, indexed by bit
const ARMING_DISABLE_FLAG_NAMES: [&str; 26] = [
    "NO_GYRO",
    "FAILSAFE",
    "RX_FAILSAFE",
    "NOT_DISARMED",
    "BOXFAILSAFE",
    "RUNAWAY_TAKEOFF",
    "CRASH_DETECTED",
    "THROTTLE",
    "ANGLE",
    "BOOT_GRACE_TIME",
    "NOPREARM",
    "LOAD",
    "CALIBRATING",
    "CLI",
    "CMS_MENU",
    "BST",
    "MSP",
    "PARALYZE",
    "GPS",
    "RESC",
    "DSHOT_TELEM",
    "REBOOT_REQUIRED",
    "DSHOT_BITBANG",
    "ACC_CALIBRATION",
    "MOTOR_PROTOCOL",
    "ARM_SWITCH",
];

/// Sensor bitmask names as reported by MSP_STATUS(_EX)
const SENSOR_NAMES: [&str; 6] = ["ACC", "BARO", "MAG", "GPS", "RANGEFINDER", "GYRO"];

/// A complete MSP response
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MspResponse {
    /// MSP command id
    pub cmd: u8,
    /// Response data
    pub data: Vec<u8>,
}

/// Flight controller health from MSP_STATUS_EX
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FcStatus {
    /// PID loop time in microseconds
    pub cycle_time_us: u16,
    /// Average system (CPU) load in percent
    pub cpu_load_pct: u16,
    /// Detected sensors
    pub sensors: Vec<String>,
    /// Why the FC refuses to arm (empty = ready to arm)
    pub arming_disable_flags: Vec<String>,
    /// FC needs a reboot to apply settings
    pub reboot_required: bool,
    /// MCU core temperature in °C (0 if unsupported)
    pub cpu_temp_c: Option<u16>,
    /// Names of the active Betaflight modes (needs MSP_BOXNAMES), e.g. ARM, ANGLE, AIR MODE
    pub active_modes: Vec<String>,
    /// Raw active-mode bitset, bit N = Nth entry of MSP_BOXNAMES
    #[serde(skip)]
    pub mode_flags: Vec<u8>,
}

/// Battery state from MSP_BATTERY_STATE
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BatteryState {
    /// Detected cell count (0 = no battery)
    pub cell_count: u8,
    /// Configured battery capacity in mAh (0 if not set)
    pub capacity_mah: u16,
    /// Pack voltage in volts (0.01 V resolution)
    pub voltage_v: f32,
    /// Average cell voltage in volts
    pub cell_voltage_v: Option<f32>,
    /// mAh drawn
    pub used_mah: u16,
    /// Current in amps (0.01 A resolution)
    pub current_a: f32,
    /// Betaflight's verdict: OK, WARNING, CRITICAL, NOT_PRESENT or INIT
    pub state: String,
}

/// Raw IMU from MSP_RAW_IMU
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Imu {
    /// Raw accelerometer ADC values (x, y, z)
    pub acc_raw: [i16; 3],
    /// Gyro rates in deg/s (x, y, z)
    pub gyro_dps: [i16; 3],
    /// Raw magnetometer values (x, y, z)
    pub mag_raw: [i16; 3],
}

/// Builds a complete CRSF frame carrying an MSPv1 request without payload
pub fn build_request(msp_cmd: u8, seq: u8) -> Vec<u8> {
    build_request_with_payload(msp_cmd, &[], seq)
}

/// Builds a complete CRSF frame carrying an MSPv1 request with a small payload (single chunk)
pub fn build_request_with_payload(msp_cmd: u8, data: &[u8], seq: u8) -> Vec<u8> {
    let status =
        STATUS_START_MASK | (MSP_V1 << STATUS_VERSION_SHIFT) | (seq & STATUS_SEQUENCE_MASK);
    let mut payload = vec![SYNC_BYTE, RADIO_ADDRESS, status, data.len() as u8, msp_cmd];
    payload.extend_from_slice(data);
    build_frame(frame_type::MSP_REQ, &payload)
}

/// Reassembles chunked MSP responses
#[derive(Debug, Default)]
pub struct MspReassembler {
    pending: Option<Pending>,
}

#[derive(Debug)]
struct Pending {
    cmd: u8,
    size: usize,
    data: Vec<u8>,
    next_seq: u8,
}

impl MspReassembler {
    /// Feeds the payload of an `MSP_RESP` CRSF frame (including dest/origin bytes)
    pub fn push(&mut self, crsf_payload: &[u8]) -> Option<MspResponse> {
        // skip DEST and ORIGIN
        let chunk = crsf_payload.get(2..)?;
        let (&status, body) = chunk.split_first()?;
        let seq = status & STATUS_SEQUENCE_MASK;

        if status & STATUS_START_MASK != 0 {
            self.pending = None;
            let version = (status & STATUS_VERSION_MASK) >> STATUS_VERSION_SHIFT;
            if status & STATUS_ERROR_MASK != 0 || version != MSP_V1 || body.len() < 2 {
                return None;
            }
            // Sizes of 255+ are "jumbo": 0xFF, CMD, then a 16-bit little-endian size
            let (size, header_len) = if body[0] == 0xFF {
                (le_u16(body.get(..4)?, 2) as usize, 4)
            } else {
                (body[0] as usize, 2)
            };
            self.pending = Some(Pending {
                size,
                cmd: body[1],
                data: Vec::with_capacity(size),
                next_seq: seq,
            });
            self.append(&body[header_len..], seq)
        } else {
            self.append(body, seq)
        }
    }

    fn append(&mut self, bytes: &[u8], seq: u8) -> Option<MspResponse> {
        let pending = self.pending.as_mut()?;
        if pending.next_seq != seq {
            self.pending = None;
            return None;
        }
        pending.next_seq = (seq + 1) & STATUS_SEQUENCE_MASK;

        let missing = pending.size - pending.data.len();
        pending
            .data
            .extend_from_slice(&bytes[..bytes.len().min(missing)]);

        if pending.data.len() == pending.size {
            let pending = self.pending.take()?;
            return Some(MspResponse {
                cmd: pending.cmd,
                data: pending.data,
            });
        }
        None
    }
}

/// Decodes MSP_MOTOR, dropping unused trailing motor slots (but keeping at least 4)
pub fn decode_motors(data: &[u8]) -> Option<Vec<u16>> {
    if data.len() < 8 {
        return None;
    }
    let motors: Vec<u16> = data.chunks_exact(2).map(|c| le_u16(c, 0)).collect();
    let used = motors.iter().rposition(|&m| m != 0).map_or(0, |i| i + 1);
    Some(motors[..used.max(4).min(motors.len())].to_vec())
}

/// Decodes MSP_BOXNAMES into the list of mode names
pub fn decode_box_names(data: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(data)
        .split(';')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

/// Names of the modes whose bit is set in `mode_flags`
pub fn active_modes(box_names: &[String], mode_flags: &[u8]) -> Vec<String> {
    box_names
        .iter()
        .enumerate()
        .filter(|(bit, _)| {
            mode_flags
                .get(bit / 8)
                .is_some_and(|b| b & (1 << (bit % 8)) != 0)
        })
        .map(|(_, name)| name.clone())
        .collect()
}

/// Decodes MSP_BATTERY_STATE
pub fn decode_battery_state(data: &[u8]) -> Option<BatteryState> {
    // cells(1) capacity(2) legacy voltage dV(1) mAh(2) current cA(2) state(1) [voltage cV(2)]
    if data.len() < 9 {
        return None;
    }
    let cell_count = data[0];
    let voltage_v = if data.len() >= 11 {
        le_u16(data, 9) as f32 / 100.
    } else {
        data[3] as f32 / 10.
    };
    Some(BatteryState {
        cell_count,
        capacity_mah: le_u16(data, 1),
        voltage_v,
        cell_voltage_v: (cell_count > 0).then(|| voltage_v / cell_count as f32),
        used_mah: le_u16(data, 4),
        current_a: le_u16(data, 6) as i16 as f32 / 100.,
        state: BATTERY_STATE_NAMES
            .get(data[8] as usize)
            .map_or_else(|| format!("STATE_{}", data[8]), |n| n.to_string()),
    })
}

/// Decodes MSP_RAW_IMU
pub fn decode_imu(data: &[u8]) -> Option<Imu> {
    if data.len() < 18 {
        return None;
    }
    let v = |i: usize| le_u16(data, i * 2) as i16;
    Some(Imu {
        acc_raw: [v(0), v(1), v(2)],
        gyro_dps: [v(3), v(4), v(5)],
        mag_raw: [v(6), v(7), v(8)],
    })
}

/// Decodes MSP_STATUS_EX
pub fn decode_status_ex(data: &[u8]) -> Option<FcStatus> {
    // cycle(2) i2c(2) sensors(2) modes(4) profile(1) load(2) profileCount(1) rateProfile(1)
    if data.len() < 16 {
        return None;
    }
    let sensor_mask = le_u16(data, 4);
    let extra_mode_bytes = data[15] as usize;
    let mut mode_flags = data[6..10].to_vec();
    mode_flags.extend_from_slice(data.get(16..16 + extra_mode_bytes).unwrap_or(&[]));
    let mut status = FcStatus {
        mode_flags,
        cycle_time_us: le_u16(data, 0),
        cpu_load_pct: le_u16(data, 11),
        sensors: SENSOR_NAMES
            .iter()
            .enumerate()
            .filter(|(bit, _)| sensor_mask & (1 << bit) != 0)
            .map(|(_, name)| name.to_string())
            .collect(),
        ..Default::default()
    };

    // extra flight-mode bytes, then arming-disable count + flags
    let mut i = 16 + extra_mode_bytes;
    if data.len() >= i + 5 {
        let count = data[i] as usize;
        let flags = u32::from_le_bytes([data[i + 1], data[i + 2], data[i + 3], data[i + 4]]);
        status.arming_disable_flags = (0..count.min(32))
            .filter(|bit| flags & (1 << bit) != 0)
            .map(|bit| {
                ARMING_DISABLE_FLAG_NAMES
                    .get(bit)
                    .map_or_else(|| format!("FLAG_{bit}"), |n| n.to_string())
            })
            .collect();
        i += 5;
    }
    if let Some(&flags) = data.get(i) {
        status.reboot_required = flags & 1 != 0;
        i += 1;
    }
    if data.len() >= i + 2 {
        status.cpu_temp_c = Some(le_u16(data, i));
    }

    Some(status)
}

fn le_u16(p: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([p[i], p[i + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fc_comms::crsf::{CrsfFrame, CrsfParser};

    fn response_chunk(status: u8, body: &[u8]) -> Vec<u8> {
        let mut payload = vec![RADIO_ADDRESS, SYNC_BYTE, status];
        payload.extend_from_slice(body);
        payload
    }

    #[test]
    fn request_frame_layout() {
        let frame = build_request(cmd::MOTOR, 3);
        let frames = CrsfParser::default().push(&frame);
        assert_eq!(
            frames,
            vec![CrsfFrame {
                frame_type: frame_type::MSP_REQ,
                payload: vec![SYNC_BYTE, RADIO_ADDRESS, 0x33, 0, cmd::MOTOR],
            }]
        );
    }

    #[test]
    fn reassembles_single_chunk() {
        let mut r = MspReassembler::default();
        let resp = r.push(&response_chunk(0x30, &[3, cmd::MOTOR, 1, 2, 3]));
        assert_eq!(
            resp,
            Some(MspResponse {
                cmd: cmd::MOTOR,
                data: vec![1, 2, 3]
            })
        );
    }

    #[test]
    fn reassembles_multiple_chunks_and_ignores_padding() {
        let mut r = MspReassembler::default();
        assert!(r.push(&response_chunk(0x3F, &[5, 150, 1, 2])).is_none());
        let resp = r.push(&response_chunk(0x20, &[3, 4, 5, 0, 0])).unwrap();
        assert_eq!(resp.cmd, 150);
        assert_eq!(resp.data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn drops_message_on_sequence_gap_or_error() {
        let mut r = MspReassembler::default();
        assert!(r.push(&response_chunk(0x31, &[5, 150, 1, 2])).is_none());
        assert!(r.push(&response_chunk(0x23, &[3, 4, 5])).is_none());
        assert!(r.push(&response_chunk(0x24, &[3, 4, 5])).is_none());
        assert!(r.push(&response_chunk(0xB0, &[0, 150])).is_none());
    }

    #[test]
    fn request_with_payload_layout() {
        let frame = build_request_with_payload(cmd::REBOOT, &[REBOOT_MODE_FIRMWARE], 2);
        let frames = CrsfParser::default().push(&frame);
        assert_eq!(
            frames[0].payload,
            vec![SYNC_BYTE, RADIO_ADDRESS, 0x32, 1, cmd::REBOOT, 0]
        );
    }

    #[test]
    fn reassembles_jumbo_response() {
        let mut r = MspReassembler::default();
        let data: Vec<u8> = (0..300u16).map(|i| i as u8).collect();
        let mut first = vec![0xFF, cmd::BOXNAMES];
        first.extend_from_slice(&300u16.to_le_bytes());
        first.extend_from_slice(&data[..50]);
        assert!(r.push(&response_chunk(0x30, &first)).is_none());
        let mut seq = 1;
        let mut resp = None;
        for chunk in data[50..].chunks(56) {
            resp = r.push(&response_chunk(0x20 | seq, chunk));
            seq += 1;
        }
        let resp = resp.unwrap();
        assert_eq!(resp.cmd, cmd::BOXNAMES);
        assert_eq!(resp.data, data);
    }

    #[test]
    fn maps_mode_flags_to_box_names() {
        let names = decode_box_names(b"ARM;ANGLE;HORIZON;AIR MODE;");
        assert_eq!(names, vec!["ARM", "ANGLE", "HORIZON", "AIR MODE"]);
        assert_eq!(active_modes(&names, &[0b1010]), vec!["ANGLE", "AIR MODE"]);
        assert!(active_modes(&names, &[]).is_empty());
    }

    #[test]
    fn decodes_battery_state() {
        let mut d = vec![4]; // 4S
        d.extend_from_slice(&1500u16.to_le_bytes()); // capacity
        d.push(152); // legacy 15.2 V
        d.extend_from_slice(&320u16.to_le_bytes()); // mAh drawn
        d.extend_from_slice(&1234u16.to_le_bytes()); // 12.34 A
        d.push(1); // WARNING
        d.extend_from_slice(&1524u16.to_le_bytes()); // 15.24 V
        let b = decode_battery_state(&d).unwrap();
        assert_eq!(b.cell_count, 4);
        assert_eq!(b.capacity_mah, 1500);
        assert!((b.voltage_v - 15.24).abs() < 1e-4);
        assert!((b.cell_voltage_v.unwrap() - 3.81).abs() < 1e-4);
        assert_eq!(b.used_mah, 320);
        assert!((b.current_a - 12.34).abs() < 1e-4);
        assert_eq!(b.state, "WARNING");

        // older firmware without the precise voltage, no battery detected
        let b = decode_battery_state(&[0, 0, 0, 0, 0, 0, 0, 0, 3]).unwrap();
        assert_eq!(b.cell_voltage_v, None);
        assert_eq!(b.state, "NOT_PRESENT");
    }

    #[test]
    fn decodes_motors_trimming_unused_slots() {
        let mut data = vec![];
        for m in [1100u16, 1200, 1300, 1400, 0, 0, 0, 0] {
            data.extend_from_slice(&m.to_le_bytes());
        }
        assert_eq!(decode_motors(&data), Some(vec![1100, 1200, 1300, 1400]));
    }

    #[test]
    fn decodes_imu() {
        let mut data = vec![];
        for v in [512i16, -3, 7, 10, -20, 30, 0, 0, 0] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let imu = decode_imu(&data).unwrap();
        assert_eq!(imu.acc_raw, [512, -3, 7]);
        assert_eq!(imu.gyro_dps, [10, -20, 30]);
    }

    #[test]
    fn decodes_status_ex() {
        let mut d = vec![];
        d.extend_from_slice(&125u16.to_le_bytes()); // cycle time
        d.extend_from_slice(&0u16.to_le_bytes()); // i2c errors
        d.extend_from_slice(&0b100001u16.to_le_bytes()); // ACC + GYRO
        d.extend_from_slice(&0u32.to_le_bytes()); // flight mode flags
        d.push(0); // pid profile
        d.extend_from_slice(&23u16.to_le_bytes()); // cpu load
        d.push(4); // pid profile count
        d.push(0); // rate profile
        d.push(1); // extra flight mode byte count
        d.push(0); // extra flight mode bytes
        d.push(26); // arming disable flags count
        d.extend_from_slice(&((1u32 << 7) | (1 << 25)).to_le_bytes());
        d.push(1); // reboot required
        d.extend_from_slice(&41u16.to_le_bytes()); // core temp

        let s = decode_status_ex(&d).unwrap();
        assert_eq!(s.cycle_time_us, 125);
        assert_eq!(s.cpu_load_pct, 23);
        assert_eq!(s.sensors, vec!["ACC", "GYRO"]);
        assert_eq!(s.arming_disable_flags, vec!["THROTTLE", "ARM_SWITCH"]);
        assert!(s.reboot_required);
        assert_eq!(s.cpu_temp_c, Some(41));
        assert_eq!(s.mode_flags, vec![0, 0, 0, 0, 0]);
    }
}
