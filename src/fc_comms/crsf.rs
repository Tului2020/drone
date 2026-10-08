//! CRSF (Crossfire) framing: encoding RC frames for the FC and parsing frames coming back from it.
//!
//! Frame layout: `[SYNC/ADDR][LEN][TYPE][PAYLOAD...][CRC]` where `LEN` counts `TYPE + PAYLOAD + CRC`
//! and the CRC-8 (DVB-S2) covers `TYPE + PAYLOAD`.

/// Sync byte / address of the flight controller
pub const SYNC_BYTE: u8 = 0xC8;
/// CRC-8 DVB-S2 polynomial
const POLY: u8 = 0xD5;
/// Payload length of an RC channels frame (16 channels × 11 bits)
pub const PAYLOAD_LEN_RC: usize = 22;
/// Smallest valid frame (SYNC + LEN + TYPE + CRC)
const MIN_FRAME_LEN: usize = 4;
/// Largest valid frame
const MAX_FRAME_LEN: usize = 64;
/// Addresses a frame can start with (FC/sync, radio, receiver, transmitter)
const VALID_ADDRESSES: [u8; 4] = [SYNC_BYTE, 0xEA, 0xEC, 0xEE];

/// CRSF frame types used by this application
pub mod frame_type {
    /// GPS
    pub const GPS: u8 = 0x02;
    /// Variometer (vertical speed)
    pub const VARIO: u8 = 0x07;
    /// Battery sensor
    pub const BATTERY_SENSOR: u8 = 0x08;
    /// Barometric altitude + vertical speed
    pub const BARO_ALTITUDE: u8 = 0x09;
    /// Heartbeat
    pub const HEARTBEAT: u8 = 0x0B;
    /// Link statistics
    pub const LINK_STATISTICS: u8 = 0x14;
    /// RC channels
    pub const RC_CHANNELS_PACKED: u8 = 0x16;
    /// Attitude
    pub const ATTITUDE: u8 = 0x1E;
    /// Flight mode text
    pub const FLIGHT_MODE: u8 = 0x21;
    /// Device info
    pub const DEVICE_INFO: u8 = 0x29;
    /// MSP request (extended frame)
    pub const MSP_REQ: u8 = 0x7A;
    /// MSP response (extended frame)
    pub const MSP_RESP: u8 = 0x7B;
}

/// A validated CRSF frame
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrsfFrame {
    /// Frame type
    pub frame_type: u8,
    /// Payload (without type and CRC)
    pub payload: Vec<u8>,
}

/// 1000-2000 µs PWM -> 172-1811 CRSF units
pub fn us_to_crsf(val_us: u16) -> u16 {
    (((val_us.saturating_sub(988)) as u32 * (1811 - 172)) / (2012 - 988) + 172) as u16
}

/// CRC-8 DVB-S2
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ POLY
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Pack up to 16 channel values (CRSF units, 11 bits each) into the 22-byte RC payload
pub fn pack_rc(ch: &[u16]) -> [u8; PAYLOAD_LEN_RC] {
    let mut out = [0u8; PAYLOAD_LEN_RC];
    let mut bit_ofs = 0;

    for &v in ch.iter().take(16) {
        let v = v & 0x07FF; // 11 bits
        let byte_idx = bit_ofs / 8;
        let bit_idx = bit_ofs % 8;

        out[byte_idx] |= ((v << bit_idx) & 0xFF) as u8;
        out[byte_idx + 1] |= ((v >> (8 - bit_idx)) & 0xFF) as u8;
        if bit_idx >= 6 {
            out[byte_idx + 2] |= ((v >> (16 - bit_idx)) & 0xFF) as u8;
        }

        bit_ofs += 11;
    }

    out
}

/// Build a complete CRSF frame
pub fn build_frame(frame_type: u8, payload: &[u8]) -> Vec<u8> {
    let length_field = payload.len() as u8 + 2; // TYPE + PAYLOAD + CRC
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&[SYNC_BYTE, length_field, frame_type]);
    frame.extend_from_slice(payload);
    frame.push(crc8(&frame[2..]));
    frame
}

/// Streaming CRSF parser. Feed it raw serial bytes and it returns every valid frame.
///
/// A frame must start with a known CRSF address, have a plausible length (4-64 bytes)
/// and a valid CRC; otherwise one byte is dropped at a time to resync on garbage.
#[derive(Debug, Default)]
pub struct CrsfParser {
    buf: Vec<u8>,
    /// Number of frames that failed the CRC check
    pub crc_errors: u64,
}

impl CrsfParser {
    /// Feed bytes into the parser and return all complete, valid frames
    pub fn push(&mut self, bytes: &[u8]) -> Vec<CrsfFrame> {
        self.buf.extend_from_slice(bytes);
        let mut frames = vec![];

        while self.buf.len() >= 2 {
            let frame_len = self.buf[1] as usize + 2;
            if !VALID_ADDRESSES.contains(&self.buf[0])
                || !(MIN_FRAME_LEN..=MAX_FRAME_LEN).contains(&frame_len)
            {
                self.buf.remove(0);
                continue;
            }
            if self.buf.len() < frame_len {
                break;
            }

            let frame = &self.buf[..frame_len];
            if crc8(&frame[2..frame_len - 1]) == frame[frame_len - 1] {
                frames.push(CrsfFrame {
                    frame_type: frame[2],
                    payload: frame[3..frame_len - 1].to_vec(),
                });
                self.buf.drain(..frame_len);
            } else {
                self.crc_errors += 1;
                self.buf.remove(0);
            }
        }

        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_then_parse_round_trips() {
        let frame = build_frame(frame_type::ATTITUDE, &[1, 2, 3, 4, 5, 6]);
        let mut parser = CrsfParser::default();
        let frames = parser.push(&frame);
        assert_eq!(
            frames,
            vec![CrsfFrame {
                frame_type: frame_type::ATTITUDE,
                payload: vec![1, 2, 3, 4, 5, 6]
            }]
        );
    }

    #[test]
    fn parser_handles_split_input() {
        let frame = build_frame(frame_type::BATTERY_SENSOR, &[0, 1, 2, 3, 4, 5, 6, 7]);
        let mut parser = CrsfParser::default();
        assert!(parser.push(&frame[..5]).is_empty());
        assert_eq!(parser.push(&frame[5..]).len(), 1);
    }

    #[test]
    fn parser_resyncs_after_garbage_and_bad_crc() {
        let good = build_frame(frame_type::FLIGHT_MODE, b"ACRO\0");
        let mut bad = build_frame(frame_type::ATTITUDE, &[9; 6]);
        *bad.last_mut().unwrap() ^= 0xFF;

        let mut input = vec![0xFF, 0x00, 0x13];
        input.extend_from_slice(&bad);
        input.extend_from_slice(&good);

        let mut parser = CrsfParser::default();
        let frames = parser.push(&input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].frame_type, frame_type::FLIGHT_MODE);
        assert!(parser.crc_errors >= 1);
    }

    #[test]
    fn us_to_crsf_matches_reference_points() {
        assert_eq!(us_to_crsf(988), 172);
        assert_eq!(us_to_crsf(2012), 1811);
        assert_eq!(us_to_crsf(1500), 991);
    }
}
