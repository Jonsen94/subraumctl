//! Control CLI and TUI for the subraum MQTT system.
//!
//! The crate is split into a transport/domain layer (`action`, `mqtt`,
//! `smoothiesaur`, `decode`) and a presentation layer (`cli`, `commands`,
//! `tui`). The CLI and TUI build the same [`action::ControlCommand`] values so
//! both front ends send byte-identical wire messages.

pub mod action;
pub mod color;
pub mod decode;
pub mod knob;
pub mod mqtt;
pub mod smoothiesaur;
pub mod white_light;

pub const APP_NAME: &str = "subraumctl";
