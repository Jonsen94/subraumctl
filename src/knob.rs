//! Animation knob metadata.
//!
//! The web UI drove 29 sliders on `knob/get/<name>` / `knob/set/<name>`.
//! The ranges and "open" flags are copied from that UI.

use clap::ValueEnum;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Knob {
    #[value(name = "cx")]
    Cx,
    #[value(name = "cy")]
    Cy,
    #[value(name = "cz")]
    Cz,
    #[value(name = "d")]
    D,
    #[value(name = "k")]
    K,
    #[value(name = "phase")]
    Phase,
    #[value(name = "saturation")]
    Saturation,
    #[value(name = "intensity")]
    Intensity,
    #[value(name = "attack")]
    Attack,
    #[value(name = "decay")]
    Decay,
    #[value(name = "sustain")]
    Sustain,
    #[value(name = "release")]
    Release,
    #[value(name = "drop")]
    Drop,
    #[value(name = "feed")]
    Feed,
    #[value(name = "chaos")]
    Chaos,
    #[value(name = "order")]
    Order,
    #[value(name = "spatial")]
    Spatial,
    #[value(name = "logical")]
    Logical,
    #[value(name = "harmony")]
    Harmony,
    #[value(name = "twitch")]
    Twitch,
    #[value(name = "exponent")]
    Exponent,
    #[value(name = "lead")]
    Lead,
    #[value(name = "pulseLead", alias = "pulse-lead")]
    PulseLead,
    #[value(name = "spread")]
    Spread,
    #[value(name = "glow")]
    Glow,
    #[value(name = "speedMin", alias = "speed-min")]
    SpeedMin,
    #[value(name = "speedRange", alias = "speed-range")]
    SpeedRange,
    #[value(name = "flowRate", alias = "flow-rate")]
    FlowRate,
    #[value(name = "loss")]
    Loss,
}

#[derive(Copy, Clone, Debug)]
pub struct KnobSpec {
    pub knob: Knob,
    pub name: &'static str,
    pub min: f64,
    pub max: f64,
    pub open_min: bool,
    pub open_max: bool,
}

pub const KNOB_SPECS: [KnobSpec; 29] = [
    KnobSpec {
        knob: Knob::Cx,
        name: "cx",
        min: -1000.0,
        max: 1000.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Cy,
        name: "cy",
        min: -1000.0,
        max: 1000.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Cz,
        name: "cz",
        min: -1000.0,
        max: 1000.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::D,
        name: "d",
        min: 0.0,
        max: 1.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::K,
        name: "k",
        min: 0.0,
        max: 1000.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Phase,
        name: "phase",
        min: 0.0,
        max: 360.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Saturation,
        name: "saturation",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: false,
    },
    KnobSpec {
        knob: Knob::Intensity,
        name: "intensity",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: false,
    },
    KnobSpec {
        knob: Knob::Attack,
        name: "attack",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Decay,
        name: "decay",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Sustain,
        name: "sustain",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Release,
        name: "release",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Drop,
        name: "drop",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Feed,
        name: "feed",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Chaos,
        name: "chaos",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Order,
        name: "order",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Spatial,
        name: "spatial",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Logical,
        name: "logical",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Harmony,
        name: "harmony",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Twitch,
        name: "twitch",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Exponent,
        name: "exponent",
        min: 0.0,
        max: 4.0,
        open_min: true,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Lead,
        name: "lead",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::PulseLead,
        name: "pulseLead",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Spread,
        name: "spread",
        min: 0.0,
        max: 1000.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Glow,
        name: "glow",
        min: 0.0,
        max: 1000.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::SpeedMin,
        name: "speedMin",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::SpeedRange,
        name: "speedRange",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::FlowRate,
        name: "flowRate",
        min: 0.0,
        max: 2.0,
        open_min: false,
        open_max: true,
    },
    KnobSpec {
        knob: Knob::Loss,
        name: "loss",
        min: 0.0,
        max: 1.0,
        open_min: false,
        open_max: true,
    },
];

impl Knob {
    pub fn index(self) -> usize {
        KNOB_SPECS
            .iter()
            .position(|spec| spec.knob == self)
            .expect("every Knob variant has a spec")
    }

    pub fn spec(self) -> &'static KnobSpec {
        &KNOB_SPECS[self.index()]
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn all() -> &'static [KnobSpec] {
        &KNOB_SPECS
    }

    pub fn from_name(name: &str) -> Option<Self> {
        KNOB_SPECS
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.knob)
    }
}

impl KnobSpec {
    pub fn step_small(&self) -> f64 {
        ((self.max - self.min) * 0.01).max(0.01)
    }

    pub fn step_big(&self) -> f64 {
        (self.max - self.min) * 0.25
    }

    pub fn ratio(&self, value: f64) -> f64 {
        let span = self.max - self.min;
        if span == 0.0 {
            0.0
        } else {
            ((value - self.min) / span).clamp(0.0, 1.0)
        }
    }

    pub fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }
}

pub fn format_value(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    if (value - value.round()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_a_spec_in_order() {
        for (index, spec) in KNOB_SPECS.iter().enumerate() {
            assert_eq!(spec.knob.index(), index);
        }
        assert_eq!(KNOB_SPECS.len(), 29);
    }

    #[test]
    fn names_round_trip() {
        for spec in KNOB_SPECS {
            assert_eq!(Knob::from_name(spec.name), Some(spec.knob));
        }
    }

    #[test]
    fn steps_match_the_web_ui() {
        assert!((Knob::Cx.spec().step_small() - 20.0).abs() < 1e-9);
        assert!((Knob::Saturation.spec().step_small() - 0.01).abs() < 1e-9);
        assert!((Knob::Exponent.spec().step_big() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn value_formatting_is_compact() {
        assert_eq!(format_value(1.0), "1");
        assert_eq!(format_value(0.5), "0.50");
        assert_eq!(format_value(-20.0), "-20");
    }
}
