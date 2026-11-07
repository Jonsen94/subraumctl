//! Smoothiesaur HTTP commands and MQTT status decoding.

use std::time::{Duration, Instant};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::action::SmoothieRelay;
use crate::decode::{
    FlowEstimator, LaserReadouts, laser_readouts, parse_bool, temperature_celsius,
};

pub const DEFAULT_SMOOTHIE_URL: &str = "http://172.31.64.137/command";
pub const RELAY_TOPIC_PREFIX: &str = "smoothie/relays/";
pub const FLOW_TOPIC: &str = "smoothie/coolant/flow";
pub const SAFETY_TOPIC: &str = "smoothie/safetyLoop";
pub const LASER_TOPIC: &str = "smoothie/laserpower";

pub const TEMP_TOPICS: [(&str, &str); 5] = [
    (
        "smoothie/mexicanTemp/relayControl",
        "mexican: relay control",
    ),
    (
        "smoothie/mexicanTemp/coolantSensors",
        "mexican: coolant sensor",
    ),
    ("smoothie/coolant/reservoir", "reservoir: body"),
    ("smoothie/coolant/primary", "reservoir: primary"),
    ("smoothie/coolant/secondary", "reservoir: secondary"),
];

/// Bit mask, label, and the web UI's inverted meaning (set bit = off).
pub const SAFETY_BITS: [(u32, &str); 8] = [
    (8, "main - port"),
    (16, "main - starboard"),
    (1, "port hatch - medial"),
    (2, "port hatch - lateral"),
    (128, "starboard hatch - medial"),
    (64, "starboard hatch - lateral"),
    (4, "aft - port"),
    (32, "aft - starboard"),
];

#[derive(Clone, Debug)]
pub struct SmoothieStatus {
    pub relays: [Option<bool>; 7],
    pub temps: [Option<f64>; 5],
    pub flow_raw: Option<i64>,
    pub flow_rate: Option<f64>,
    pub flow: FlowEstimator,
    pub safety: Option<u32>,
    pub laser: LaserReadouts,
}

impl Default for SmoothieStatus {
    fn default() -> Self {
        Self {
            relays: [None; 7],
            temps: [None; 5],
            flow_raw: None,
            flow_rate: None,
            flow: FlowEstimator::default(),
            safety: None,
            laser: LaserReadouts::default(),
        }
    }
}

impl SmoothieStatus {
    pub fn apply(&mut self, topic: &str, payload: &str, at: Instant) {
        if let Some(name) = topic.strip_prefix(RELAY_TOPIC_PREFIX) {
            if let Some(relay) = SmoothieRelay::from_topic_name(name) {
                self.relays[relay_index(relay)] = Some(parse_bool(payload));
            }
            return;
        }

        if let Some(index) = TEMP_TOPICS
            .iter()
            .position(|(temp_topic, _)| *temp_topic == topic)
        {
            self.temps[index] = temperature_celsius(payload);
            return;
        }

        if topic == FLOW_TOPIC {
            if let Ok(raw) = payload.trim().parse::<i64>() {
                self.flow_raw = Some(raw);
                self.flow_rate = Some(self.flow.update(raw, at));
            }
        } else if topic == SAFETY_TOPIC {
            self.safety = parse_hex(payload);
        } else if topic == LASER_TOPIC {
            self.laser = laser_readouts(payload);
        }
    }

    pub fn relay(&self, relay: SmoothieRelay) -> Option<bool> {
        self.relays[relay_index(relay)]
    }

    pub fn safety_bits(&self) -> Vec<(u32, &'static str, bool)> {
        match self.safety {
            Some(mask) => SAFETY_BITS
                .iter()
                .map(|(bit, label)| (*bit, *label, (mask & bit) == 0))
                .collect(),
            None => SAFETY_BITS
                .iter()
                .map(|(bit, label)| (*bit, *label, false))
                .collect(),
        }
    }

    pub fn reset_flow(&mut self) {
        self.flow_raw = None;
        self.flow_rate = None;
        self.flow.reset();
    }
}

pub fn relay_index(relay: SmoothieRelay) -> usize {
    SmoothieRelay::ALL
        .iter()
        .position(|candidate| *candidate == relay)
        .expect("every SmoothieRelay has an index")
}

fn parse_hex(payload: &str) -> Option<u32> {
    let hex: String = payload
        .trim()
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .collect();
    if hex.is_empty() {
        None
    } else {
        u32::from_str_radix(&hex, 16).ok()
    }
}

#[derive(Clone, Debug)]
pub struct HttpCommand {
    pub url: String,
    pub body: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct HttpEvent {
    pub body: String,
    pub description: String,
    pub result: Result<(), String>,
}

pub fn spawn_http_worker(
    mut command_rx: UnboundedReceiver<HttpCommand>,
    event_tx: UnboundedSender<HttpEvent>,
    timeout: Duration,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        while let Some(command) = command_rx.recv().await {
            let result = match client
                .post(&command.url)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(command.body.clone())
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => Ok(()),
                Ok(response) => Err(format!("HTTP status {}", response.status())),
                Err(err) => Err(err.to_string()),
            };
            let _ = event_tx.send(HttpEvent {
                body: command.body,
                description: command.description,
                result,
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_status_is_decoded() {
        let mut status = SmoothieStatus::default();
        status.apply("smoothie/relays/pumpPrimary", "on", Instant::now());
        assert_eq!(status.relay(SmoothieRelay::PumpPrimary), Some(true));
        status.apply("smoothie/relays/fanPort", "0", Instant::now());
        assert_eq!(status.relay(SmoothieRelay::FanPort), Some(false));
    }

    #[test]
    fn temperatures_and_safety_are_decoded() {
        let mut status = SmoothieStatus::default();
        status.apply("smoothie/coolant/reservoir", "296.3", Instant::now());
        assert!((status.temps[2].unwrap() - 23.15).abs() < 1e-9);
        status.apply(SAFETY_TOPIC, "8", Instant::now());
        assert_eq!(status.safety, Some(8));
        let bits = status.safety_bits();
        assert_eq!(bits[0], (8, "main - port", false));
        assert_eq!(bits[1], (16, "main - starboard", true));
    }

    #[test]
    fn flow_is_smoothed() {
        let mut status = SmoothieStatus::default();
        let t0 = Instant::now();
        status.apply(FLOW_TOPIC, "0", t0);
        status.apply(FLOW_TOPIC, "100", t0 + Duration::from_secs(1));
        assert_eq!(status.flow_raw, Some(100));
        assert!((status.flow_rate.unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn laser_readouts_are_decoded() {
        let mut status = SmoothieStatus::default();
        status.apply(LASER_TOPIC, "255 65535 0", Instant::now());
        assert!((status.laser.control_8.unwrap() - 100.0).abs() < 1e-9);
        assert!((status.laser.control_16.unwrap() - 100.0).abs() < 1e-9);
        assert!(status.laser.volts.is_some());
    }

    #[test]
    fn every_safety_bit_has_a_label() {
        assert_eq!(SAFETY_BITS.len(), 8);
        for (_, label) in SAFETY_BITS {
            assert!(!label.is_empty());
        }
    }
}
