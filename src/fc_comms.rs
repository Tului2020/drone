//! Module for FC communications
mod rc_controls;

use std::{
    sync::{Arc, Mutex},
    thread::{sleep, spawn as thread_spawn},
    time::Duration,
};

use rc_controls::RcControls;
use serialport::SerialPort;
use tracing::{debug, error};

use crate::{app_data::DroneAppData, error::DroneError, DroneResult};

const SYNC_BYTE: u8 = 0xC8; // "address" / sync marker
const POLY: u8 = 0xD5; // CRC-8 DVB-S2 polynomial
const TYPE_RC: u8 = 0x16;
// const TYPE_BATT: u8 = 0x08;
// const TYPE_GPS: u8 = 0x02;
// const TYPE_ATTITUDE: u8 = 0x1E;
// const TYPE_FLIGHT_MODE: u8 = 0x21;
// const FRAME_OVERHEAD: u8 = 4;
const PAYLOAD_LEN_RC: u8 = 22; // 16 × 11 bit = 176 bit = 22 byte

/// FC communications
pub struct FcComms {
    /// Serial port
    port: Arc<Mutex<Box<dyn SerialPort + 'static>>>,
    /// RC controls
    rc_controls: Arc<Mutex<RcControls>>,
}

impl FcComms {
    /// Create a new instance of the FC communications
    pub fn new(app_data: &DroneAppData) -> DroneResult<Self> {
        let port_name = app_data.fc_port_name();
        let baud_rate = app_data.fc_baud_rate();
        let port = Arc::new(Mutex::new(
            serialport::new(port_name, baud_rate)
                .timeout(Duration::from_millis(1000))
                .open()
                .map_err(|e| {
                    error!("{e}");
                    DroneError::SerialPort("Could not open port".to_string())
                })?,
        ));
        debug!("Serial port opened: {port_name} at {baud_rate} baud");

        let rc_controls = Arc::new(Mutex::new(RcControls::default()));

        let rc_controls_clone = rc_controls.clone();
        let port_clone = port.clone();
        thread_spawn(move || loop {
            {
                let chans_us = { rc_controls_clone.lock().unwrap().chans_us() };
                let chans: Vec<u16> = chans_us.iter().map(|&x| us_to_crsf(x)).collect();
                let payload = pack_rc(&chans);
                let frame = build_frame(TYPE_RC, &payload);

                let _s = port_clone.lock().unwrap().write_all(&frame).map_err(|e| {
                    error!("Failed to send RC data: {e}");
                    DroneError::SerialPort("Failed to send RC data".to_string())
                });
            }

            sleep(Duration::from_millis(20));
        });

        Ok(Self { port, rc_controls })
    }

    /// Send RC data
    pub fn set_rc_controls(
        &mut self,
        roll: Option<u16>,
        pitch: Option<u16>,
        thr: Option<u16>,
        yaw: Option<u16>,
        aux1: Option<u16>,
        aux2: Option<u16>,
        aux3: Option<u16>,
        aux4: Option<u16>,
    ) {
        let mut rc_controls = self.rc_controls.lock().unwrap();

        if let Some(roll) = roll {
            rc_controls.roll = roll;
        }
        if let Some(pitch) = pitch {
            rc_controls.pitch = pitch;
        }
        if let Some(yaw) = yaw {
            rc_controls.yaw = yaw;
        }
        if let Some(thr) = thr {
            rc_controls.thr = thr;
        }
        if let Some(aux1) = aux1 {
            rc_controls.aux1 = aux1;
        }
        if let Some(aux2) = aux2 {
            rc_controls.aux2 = aux2;
        }
        if let Some(aux3) = aux3 {
            rc_controls.aux3 = aux3;
        }
        if let Some(aux4) = aux4 {
            rc_controls.aux4 = aux4;
        }
    }

    /// Listen to the serial port
    pub fn listen(&mut self) -> DroneResult<()> {
        let mut buffer = [0u8; 1024];
        loop {
            match self.port.lock().unwrap().read(&mut buffer) {
                Ok(n) => {
                    if n > 0 {
                        debug!("Received: {:?}", &buffer[..n]);
                        // Process the received data
                    }
                }
                Err(e) => {
                    error!("Read failed: {e}");
                    return Err(DroneError::SerialPort("Read failed".to_string()));
                }
            }
        }
    }
}

/// Helper function
fn us_to_crsf(val_us: u16) -> u16 {
    (((val_us.saturating_sub(988)) as u32 * (1811 - 172)) / (2012 - 988) + 172) as u16
}

/// Hash function
fn crc8(data: &[u8]) -> u8 {
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

/// Pack RC data
fn pack_rc(ch: &[u16]) -> [u8; 22] {
    let mut out = [0u8; PAYLOAD_LEN_RC as usize];
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

fn build_frame(frame_type: u8, payload: &[u8]) -> Vec<u8> {
    let length_field = payload.len() as u8 + 2; // TYPE + PAYLOAD + CRC
    let mut hdr = vec![SYNC_BYTE, length_field, frame_type];

    let crc = crc8(
        &[frame_type]
            .iter()
            .chain(payload.iter())
            .copied()
            .collect::<Vec<u8>>(),
    );
    hdr.extend_from_slice(payload);
    hdr.push(crc);

    hdr
}
