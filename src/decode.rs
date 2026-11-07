//! Decoders for the payloads seen in the web UI.

use std::time::Instant;

/// Boolean parser mirroring the JS `parseBool` helper closely.
pub fn parse_bool(payload: &str) -> bool {
    let value = payload.trim();
    if value.is_empty() {
        return false;
    }
    match value.to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "y" => true,
        "false" | "off" | "no" | "n" => false,
        other => other.parse::<f64>().map(|n| n != 0.0).unwrap_or(true),
    }
}

pub fn temperature_celsius(payload: &str) -> Option<f64> {
    payload
        .trim()
        .parse::<f64>()
        .ok()
        .map(|kelvin| kelvin - 273.15)
}

pub fn laser_control_8(payload: &str) -> Option<f64> {
    let raw = payload.split_whitespace().next()?.parse::<f64>().ok()?;
    Some(raw * 100.0 / 255.0)
}

pub fn laser_control_16(payload: &str) -> Option<f64> {
    let raw = payload
        .split_whitespace()
        .nth(1)?
        .parse::<f64>()
        .ok()?;
    Some(raw * 100.0 / 65535.0)
}

/// Returns `(percent_of_2_7_v, volts)` for the third laser-power value.
pub fn laser_voltage(payload: &str) -> Option<(f64, f64)> {
    let raw = payload
        .split_whitespace()
        .nth(2)?
        .parse::<f64>()
        .ok()?;
    let volts = (raw + 200.0) * 6.430107526881721e-05;
    Some((volts / 2.7 * 100.0, volts))
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LaserReadouts {
    pub control_8: Option<f64>,
    pub control_16: Option<f64>,
    pub percent: Option<f64>,
    pub volts: Option<f64>,
}

pub fn laser_readouts(payload: &str) -> LaserReadouts {
    LaserReadouts {
        control_8: laser_control_8(payload),
        control_16: laser_control_16(payload),
        percent: laser_voltage(payload).map(|(percent, _)| percent),
        volts: laser_voltage(payload).map(|(_, volts)| volts),
    }
}

/// Safety switches are inverted in the web UI: a set bit means "off".
pub fn safety_bit(payload: &str, mask: u32) -> Option<bool> {
    let hex: String = payload
        .trim()
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .collect();
    if hex.is_empty() {
        return None;
    }
    let value = u32::from_str_radix(&hex, 16).ok()?;
    Some((value & mask) == 0)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlertCondition {
    Green,
    Blue,
    Contact,
    Offworld,
    Intruder,
    Red,
    SelfDestruct,
    Biohazard,
    Unknown,
}

impl AlertCondition {
    pub fn from_token(token: &str) -> Self {
        match token.to_ascii_lowercase().as_str() {
            "green" => Self::Green,
            "blue" => Self::Blue,
            "contact" => Self::Contact,
            "offworld" => Self::Offworld,
            "intruder" => Self::Intruder,
            "red" => Self::Red,
            "selfdestruct" | "self-destruct" => Self::SelfDestruct,
            "biohazard" => Self::Biohazard,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Green => "Code: Green",
            Self::Blue => "Code: Blue",
            Self::Contact => "Code: Yellow",
            Self::Offworld => "Code: Purple",
            Self::Intruder | Self::Red | Self::SelfDestruct => "Code: Red",
            Self::Biohazard => "Code: Black",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AlertState {
    pub condition: AlertCondition,
    pub dim: bool,
}

pub fn parse_alert(payload: &str) -> AlertState {
    let mut tokens = payload.split_whitespace();
    let condition = tokens
        .next()
        .map(AlertCondition::from_token)
        .unwrap_or(AlertCondition::Unknown);
    let dim = tokens.any(|token| token.eq_ignore_ascii_case("dim"));
    AlertState { condition, dim }
}

/// Smoothed derivative of the coolant-flow counter.
///
/// The web UI uses `estimate = estimate * 0.9 + raw * 0.1`; the first sample
/// only primes the estimator and reports zero.
#[derive(Copy, Clone, Debug, Default)]
pub struct FlowEstimator {
    last: Option<(i64, Instant)>,
    estimate: f64,
}

impl FlowEstimator {
    pub fn update(&mut self, value: i64, at: Instant) -> f64 {
        if let Some((old_value, old_at)) = self.last {
            let elapsed = at.saturating_duration_since(old_at).as_secs_f64();
            if elapsed > 0.0 {
                let raw = (value as f64 - old_value as f64) / elapsed;
                self.estimate = self.estimate * 0.9 + raw * 0.1;
            }
        }
        self.last = Some((value, at));
        self.estimate
    }

    pub fn estimate(&self) -> f64 {
        self.estimate
    }

    pub fn reset(&mut self) {
        self.last = None;
        self.estimate = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn bool_parser_matches_the_js_helper() {
        for value in ["true", "on", "yes", "y", "TRUE", "1", "2", "garbage"] {
            assert!(parse_bool(value), "{value}");
        }
        for value in ["false", "off", "no", "n", "FALSE", "0", "", "  "] {
            assert!(!parse_bool(value), "{value:?}");
        }
    }

    #[test]
    fn temperatures_are_converted_from_kelvin() {
        assert!((temperature_celsius("296.3").unwrap() - 23.15).abs() < 1e-9);
        assert!(temperature_celsius("nope").is_none());
    }

    #[test]
    fn laser_values_are_decoded() {
        assert!((laser_control_8("255 65535 0").unwrap() - 100.0).abs() < 1e-9);
        assert!((laser_control_16("255 65535 0").unwrap() - 100.0).abs() < 1e-9);
        let (_, volts) = laser_voltage("0 0 0").unwrap();
        assert!((volts - 0.012_860_215).abs() < 1e-9);
        assert!(laser_voltage("0 0").is_none());
    }

    #[test]
    fn safety_bits_are_inverted() {
        assert_eq!(safety_bit("8", 8), Some(false));
        assert_eq!(safety_bit("0", 8), Some(true));
        assert_eq!(safety_bit("ff", 8), Some(false));
        assert_eq!(safety_bit("zz", 8), None);
    }

    #[test]
    fn alerts_are_mapped() {
        let state = parse_alert("green dim");
        assert_eq!(state.condition, AlertCondition::Green);
        assert!(state.dim);
        assert_eq!(parse_alert("selfdestruct").condition.label(), "Code: Red");
        assert_eq!(parse_alert("").condition, AlertCondition::Unknown);
    }

    #[test]
    fn flow_estimator_primes_then_smooths() {
        let mut estimator = FlowEstimator::default();
        let t0 = Instant::now();
        assert!((estimator.update(0, t0) - 0.0).abs() < 1e-9);
        let t1 = t0 + Duration::from_secs(1);
        assert!((estimator.update(100, t1) - 10.0).abs() < 1e-9);
        let t2 = t1 + Duration::from_secs(1);
        assert!((estimator.update(200, t2) - 19.0).abs() < 1e-9);
    }
}
