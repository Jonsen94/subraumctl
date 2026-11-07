//! Interactive TUI.
//!
//! This file currently provides the option type and entry point. The terminal
//! UI is added in the following commits.

use eyre::Result;

use crate::cli::TuiView;
use crate::color::{Theme, ThemePreset};
use crate::mqtt::MqttOptions;

pub struct TuiOptions {
    pub mqtt: MqttOptions,
    pub view: TuiView,
    pub connect: bool,
    pub safe_mode: bool,
    pub theme: Theme,
    pub theme_preset: ThemePreset,
    pub smoothie_url: String,
    pub verbose: u8,
    pub dry_run: bool,
}

pub async fn run(_options: TuiOptions) -> Result<()> {
    eyre::bail!("TUI is not implemented yet")
}
