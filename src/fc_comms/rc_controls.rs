//! RcControls struct
use std::fmt::Display;

/// Struct to hold the RC controls values
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
            "[{}, {}, {}, {}, {}, {}, {}, {}]",
            self.roll, self.pitch, self.yaw, self.thr, self.aux1, self.aux2, self.aux3, self.aux4
        )
    }
}
