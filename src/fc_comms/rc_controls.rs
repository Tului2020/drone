//! RcControls struct
use std::fmt::Display;

use serde::Deserialize;
use tracing::info;

/// Struct to hold the RC controls values
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct RcControls {
    /// Roll channel value
    pub roll: u16,
    /// Pitch channel value
    pub pitch: u16,
    /// Yaw channel value
    pub yaw: u16,
    /// Throttle channel value
    pub thr: u16,
    /// Auxiliary channel 1 value
    pub aux1: u16,
    /// Auxiliary channel 2 value
    pub aux2: u16,
    /// Auxiliary channel 3 value
    pub aux3: u16,
    /// Auxiliary channel 4 value
    pub aux4: u16,
}

impl RcControls {
    #[cfg(feature = "real")]
    /// Converts the RcControls struct to a byte array for real drone communication
    pub fn chans_us(&self) -> [u16; 16] {
        [
            self.roll, self.pitch, self.thr, self.yaw, self.aux1, self.aux2, self.aux3, self.aux4,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]
    }

    /// Update the RcControls struct with another RcControls struct
    pub fn update(&mut self, other: &RcControls) {
        info!("{other}");
        self.roll = other.roll;
        self.pitch = other.pitch;
        self.yaw = other.yaw;
        self.thr = other.thr;
        self.aux1 = other.aux1;
        self.aux2 = other.aux2;
        self.aux3 = other.aux3;
        self.aux4 = other.aux4;
    }

    /// Resets the RcControls struct to default values
    pub fn reset(&mut self) {
        *self = RcControls::default();
        info!("RcControls reset to default");
    }

    /// Converts the RcControls struct to a byte array for CRSF communication
    pub fn from_str(s: &str) -> RcControls {
        let mut rc = RcControls::default();
        for part in s.trim().split(',') {
            let mut kv = part.split('=');
            if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                let value = value.trim();

                match key {
                    "roll" => rc.roll = value.parse().unwrap(),
                    "pitch" => rc.pitch = value.parse().unwrap(),
                    "yaw" => rc.yaw = value.parse().unwrap(),
                    "thr" => rc.thr = value.parse().unwrap(),
                    "aux1" => rc.aux1 = value.parse().unwrap(),
                    "aux2" => rc.aux2 = value.parse().unwrap(),
                    "aux3" => rc.aux3 = value.parse().unwrap(),
                    "aux4" => rc.aux4 = value.parse().unwrap(),
                    _ => {}
                }
            }
        }
        rc
    }

    /// Converts the RcControls struct to a string
    pub fn to_str(&self) -> String {
        format!("{self}")
    }
}

impl Default for RcControls {
    fn default() -> Self {
        Self {
            roll: 1500,
            pitch: 1500,
            yaw: 1500,
            thr: 885,
            aux1: 1000,
            aux2: 1000,
            aux3: 1000,
            aux4: 1000,
        }
    }
}

impl Display for RcControls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "roll={},pitch={},yaw={},thr={},aux1={},aux2={},aux3={},aux4={}",
            self.roll, self.pitch, self.yaw, self.thr, self.aux1, self.aux2, self.aux3, self.aux4
        )
    }
}
