//! Per-axis stick sensitivity settings for the DualSense controller
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::DroneResult;

/// Sensitivity settings for one stick axis
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AxisSettings {
    /// Fraction (0-1) of the full output range reached at full stick deflection.
    /// For roll/pitch/yaw full range is ±500µs, for throttle it is the span above the base throttle.
    pub rate: f32,
    /// 0 = linear, 1 = fully cubic (soft around center, steep near the edges)
    pub expo: f32,
    /// Fraction (0-0.5) of stick travel around center that is ignored
    pub deadzone: f32,
    /// Maximum output change in µs per second (0 = unlimited). Makes sudden full presses ramp in.
    pub slew_us_per_s: f32,
}

impl AxisSettings {
    /// Applies deadzone, expo and rate to a stick value in [-1, 1]. Returns a value in [-rate, rate].
    pub fn apply(&self, stick: f32) -> f32 {
        let stick = stick.clamp(-1., 1.);
        let magnitude = stick.abs();
        if magnitude <= self.deadzone {
            return 0.;
        }

        let x = (magnitude - self.deadzone) / (1. - self.deadzone);
        let curved = (1. - self.expo) * x + self.expo * x.powi(3);
        stick.signum() * curved * self.rate
    }

    fn validate(&self, axis: &str) -> Result<(), String> {
        let check = |name: &str, value: f32, min: f32, max: f32| {
            if value.is_finite() && (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(format!(
                    "{axis}.{name} must be between {min} and {max}, got {value}"
                ))
            }
        };
        check("rate", self.rate, 0., 1.)?;
        check("expo", self.expo, 0., 1.)?;
        check("deadzone", self.deadzone, 0., 0.5)?;
        check("slew_us_per_s", self.slew_us_per_s, 0., 100_000.)
    }
}

impl Default for AxisSettings {
    /// Matches the original hard-coded behaviour: cubic curve, 10/128 deadzone, full range
    fn default() -> Self {
        Self {
            rate: 1.,
            expo: 1.,
            deadzone: 10. / 128.,
            slew_us_per_s: 0.,
        }
    }
}

/// Sensitivity settings for all stick axes
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ControllerSettings {
    /// Right stick X
    pub roll: AxisSettings,
    /// Right stick Y
    pub pitch: AxisSettings,
    /// Left stick X
    pub yaw: AxisSettings,
    /// Left stick Y
    pub throttle: AxisSettings,
}

impl ControllerSettings {
    /// Checks every value is in range
    pub fn validate(&self) -> Result<(), String> {
        self.roll.validate("roll")?;
        self.pitch.validate("pitch")?;
        self.yaw.validate("yaw")?;
        self.throttle.validate("throttle")
    }

    /// Loads settings from `path`, falling back to defaults if it is missing or invalid
    pub fn load(path: &Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            info!(
                "No controller settings at {}, using defaults",
                path.display()
            );
            return Self::default();
        };
        match serde_json::from_str::<Self>(&contents) {
            Ok(settings) if settings.validate().is_ok() => settings,
            Ok(settings) => {
                warn!(
                    "Ignoring invalid controller settings in {}: {}",
                    path.display(),
                    settings.validate().unwrap_err()
                );
                Self::default()
            }
            Err(e) => {
                warn!("Failed to parse {}: {e}. Using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Saves settings to `path` as pretty JSON
    pub fn save(&self, path: &Path) -> DroneResult {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// Limits how fast each output channel can change
#[derive(Debug, Default)]
pub struct SlewLimiter {
    current: Option<f32>,
}

impl SlewLimiter {
    /// Moves toward `target` by at most `max_us_per_s × dt_s` (0 = jump straight to target)
    pub fn step(&mut self, target: f32, max_us_per_s: f32, dt_s: f32) -> f32 {
        let next = match self.current {
            Some(current) if max_us_per_s > 0. => {
                let max_step = max_us_per_s * dt_s;
                current + (target - current).clamp(-max_step, max_step)
            }
            _ => target,
        };
        self.current = Some(next);
        next
    }

    /// Jumps straight to `value`
    pub fn reset(&mut self, value: f32) {
        self.current = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(rate: f32, expo: f32, deadzone: f32) -> AxisSettings {
        AxisSettings {
            rate,
            expo,
            deadzone,
            slew_us_per_s: 0.,
        }
    }

    #[test]
    fn linear_curve_scales_by_rate() {
        let a = axis(0.5, 0., 0.);
        assert_eq!(a.apply(1.), 0.5);
        assert_eq!(a.apply(-0.5), -0.25);
        assert_eq!(a.apply(2.), 0.5); // clamped
    }

    #[test]
    fn expo_softens_center_but_keeps_endpoints() {
        let a = axis(1., 1., 0.);
        assert_eq!(a.apply(1.), 1.);
        assert!((a.apply(0.5) - 0.125).abs() < 1e-6);
        let half = axis(1., 0.5, 0.);
        assert!((half.apply(0.5) - (0.25 + 0.0625)).abs() < 1e-6);
    }

    #[test]
    fn deadzone_zeroes_center_and_rescales() {
        let a = axis(1., 0., 0.2);
        assert_eq!(a.apply(0.1), 0.);
        assert_eq!(a.apply(-0.2), 0.);
        assert!((a.apply(0.6) - 0.5).abs() < 1e-6);
        assert_eq!(a.apply(1.), 1.);
    }

    #[test]
    fn slew_limits_change_per_step() {
        let mut s = SlewLimiter::default();
        assert_eq!(s.step(1500., 1000., 0.02), 1500.);
        assert_eq!(s.step(2000., 1000., 0.02), 1520.);
        assert_eq!(s.step(2000., 1000., 0.02), 1540.);
        assert_eq!(s.step(1000., 0., 0.02), 1000.);
        s.reset(1200.);
        assert_eq!(s.step(1000., 1000., 0.1), 1100.);
    }

    #[test]
    fn validation_rejects_out_of_range() {
        let mut settings = ControllerSettings::default();
        assert!(settings.validate().is_ok());
        settings.pitch.rate = 1.5;
        assert!(settings.validate().unwrap_err().contains("pitch.rate"));
        settings.pitch.rate = 0.5;
        settings.yaw.deadzone = f32::NAN;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn partial_json_fills_defaults() {
        let s: ControllerSettings = serde_json::from_str(
            r#"{"pitch":{"rate":0.4,"expo":0.6,"deadzone":0.1,"slew_us_per_s":800}}"#,
        )
        .unwrap();
        assert_eq!(s.pitch.rate, 0.4);
        assert_eq!(s.roll, AxisSettings::default());
    }
}
