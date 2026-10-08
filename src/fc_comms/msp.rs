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
    /// Raw accelerometer, gyro (deg/s) and magnetometer
    pub const RAW_IMU: u8 = 102;
    /// Motor outputs
    pub const MOTOR: u8 = 104;
    /// Extended status: loop time, CPU load, arming-disable flags...
    pub const STATUS_EX: u8 = 150;
}

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
    let status = STATUS_START_MASK | (MSP_V1 << STATUS_VERSION_SHIFT) | (seq & STATUS_SEQUENCE_MASK);
    build_frame(
        frame_type::MSP_REQ,
        &[SYNC_BYTE, RADIO_ADDRESS, status, 0, msp_cmd],
    )
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
            self.pending = Some(Pending {
                size: body[0] as usize,
                cmd: body[1],
                data: Vec::with_capacity(body[0] as usize),
                next_seq: seq,
            });
            self.append(&body[2..], seq)
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
    let mut status = FcStatus {
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
    let mut i = 16 + data[15] as usize;
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
    }
}
