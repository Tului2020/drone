//! State for the DualSense controller.
use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct DualSenseControllerState {
    /// Whether the controller is in flight mode.
    flight_mode: bool,
}

impl DualSenseControllerState {
    /// Sets the flight mode of the controller.
    pub fn set_flight_mode(&mut self, new_flight_mode: bool) {
        self.flight_mode = new_flight_mode;
        debug!("flight_mode: {}", self.flight_mode);
    }

    /// Returns whether the controller is in flight mode.
    pub fn flight_mode(&self) -> bool {
        self.flight_mode
    }
}
