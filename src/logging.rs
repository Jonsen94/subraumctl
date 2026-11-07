//! Tracing setup for the CLI and the TUI.
//!
//! The TUI uses `tui-logger`'s tracing subscriber layer so application logs and
//! paho-mqtt's bridged `log` records appear in the in-app log pane. Nothing is
//! written to stderr while the alternate screen is active.

use eyre::Result;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

pub fn init_cli(verbose: u8, quiet: bool) {
    let level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}

pub fn init_tui(verbose: u8) -> Result<()> {
    let level = match verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };
    tracing_subscriber::registry()
        .with(level)
        .with(tui_logger::TuiTracingSubscriberLayer)
        .try_init()
        .map_err(|err| eyre::eyre!("failed to initialize TUI logging: {err}"))?;
    Ok(())
}
