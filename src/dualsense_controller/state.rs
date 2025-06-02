//! State for the DualSense controller.
use std::fmt::Display;

use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct DualSenseControllerState {
    /// Whether the controller is in flight mode.
    flight_mode: FlightMode,
}

impl DualSenseControllerState {
    /// Sets the flight mode of the controller.
    pub fn set_flight_mode(&mut self, new_flight_mode: FlightMode) {
        self.flight_mode = new_flight_mode;
        debug!("flight_mode: {}", self.flight_mode);
    }

    /// Returns whether the controller is in flight mode.
    pub fn flight_mode(&self) -> &FlightMode {
        &self.flight_mode
    }
}

/// Represents the flight mode of the controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlightMode {
    Ready,
    Hover,
    Land,
    Custom(i16),
}

impl FlightMode {
    /// Returns the base throttle value for the flight mode.
    pub fn get_base_thr(&self) -> i16 {
        match self {
            FlightMode::Ready => 1000,
            FlightMode::Hover => 1500,
            FlightMode::Land => 1450,
            FlightMode::Custom(value) => *value,
        }
    }
}

impl Default for FlightMode {
    fn default() -> Self {
        FlightMode::Ready
    }
}

impl Display for FlightMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlightMode::Ready => write!(f, "Ready"),
            FlightMode::Hover => write!(f, "Hover"),
            FlightMode::Land => write!(f, "Land"),
            FlightMode::Custom(value) => write!(f, "Custom({})", value),
        }
    }
}
