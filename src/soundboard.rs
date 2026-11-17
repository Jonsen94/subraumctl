//! Soundboard commands over EDI AMQP, shared by callers such as the TUI.
//!
//! Build a validated [`Request`] with [`Request::search`] or [`Request::play`],
//! then await [`transport::execute`] with [`Options`] and a dry-run flag.
//! Search returns newline-separated sample names as [`Response::Search`].
//! [`Response::PlayPublished`] confirms broker acceptance, not audible playback.
//! No request is retried automatically.

use std::time::Duration;

use eyre::{Result, ensure};
use serde_json::{Value, json};

pub mod transport;

pub const DEFAULT_AMQP_URL: &str = "amqp://amqp.subraum.c3pb.de:5672/%2f";
pub const MAX_ARGUMENT_BYTES: usize = 4096;
pub const MAX_REPLY_BYTES: usize = 64 * 1024;
pub const MAX_RESULTS: usize = 500;

/// Connection settings. Debug is intentionally not derived: URLs may contain secrets.
#[derive(Clone)]
pub struct Options {
    pub amqp_url: String,
    /// Shared deadline for connection, setup, publication and the search reply.
    /// Best-effort connection cleanup may take up to two additional seconds.
    pub timeout: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            amqp_url: DEFAULT_AMQP_URL.into(),
            timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Search,
    Play,
}

/// Arguments cannot be modified after validation; spaces are preserved literally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    operation: Operation,
    args: String,
}

impl Request {
    pub fn new(operation: Operation, args: impl Into<String>) -> Result<Self> {
        let args = args.into();
        validate_text(&args)?;
        Ok(Self { operation, args })
    }

    pub fn search(query: impl Into<String>) -> Result<Self> {
        Self::new(Operation::Search, query)
    }

    pub fn play(sample: impl Into<String>) -> Result<Self> {
        Self::new(Operation::Play, sample)
    }

    pub fn command(&self) -> &'static str {
        match self.operation {
            Operation::Search => "sobfind",
            Operation::Play => "sob",
        }
    }

    pub fn args(&self) -> &str {
        &self.args
    }

    pub fn read_only(&self) -> bool {
        self.operation == Operation::Search
    }

    fn payload(&self, source: &str) -> Value {
        json!({"cmd": self.command(), "args": self.args(), "user": "subraumctl",
            "src": source, "dst": null})
    }
}

/// Finite, endpoint-free results for presentation layers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    /// No connection, queue or publication was attempted.
    DryRun { command: &'static str, args: String },
    /// At most [`MAX_RESULTS`] literal sample names, each at most 4096 bytes.
    /// `truncated` indicates additional nonempty matches beyond the limit.
    Search {
        matches: Vec<String>,
        truncated: bool,
    },
    /// Mandatory publication was routed and acknowledged by the broker.
    /// The service need not reply; this does not confirm that sound was played.
    PlayPublished,
}

fn validate_text(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty(),
        "soundboard argument must not be empty"
    );
    ensure!(
        value.len() <= MAX_ARGUMENT_BYTES,
        "soundboard argument exceeds 4096 bytes"
    );
    ensure!(
        !value.chars().any(char::is_control),
        "soundboard argument must not contain control characters"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_validated_without_changing_literal_samples() {
        for operation in [Operation::Search, Operation::Play] {
            for invalid in ["", "  ", "a\nb", "a\rb", "a\tb", "a\0b", "a\u{85}b"] {
                assert!(Request::new(operation, invalid).is_err());
            }
            assert!(Request::new(operation, "x".repeat(MAX_ARGUMENT_BYTES)).is_ok());
            assert!(Request::new(operation, "x".repeat(MAX_ARGUMENT_BYTES + 1)).is_err());
            assert!(Request::new(operation, "ä".repeat(MAX_ARGUMENT_BYTES / 2)).is_ok());
            assert!(Request::new(operation, "ä".repeat(MAX_ARGUMENT_BYTES / 2 + 1)).is_err());
        }
        let sample = "  a  sample \"quoted\" \\ literal; $()  ";
        let request = Request::play(sample).unwrap();
        assert_eq!(request.args(), sample);
        assert_eq!(request.command(), "sob");
        assert!(!request.read_only());
        assert_eq!(
            request.payload("subraumctl.recv.test"),
            json!({
                "cmd": "sob", "args": sample, "user": "subraumctl",
                "src": "subraumctl.recv.test", "dst": null
            })
        );
        let search = Request::search("a sample").unwrap();
        assert_eq!(search.command(), "sobfind");
        assert!(search.read_only());
        assert_eq!(search.payload("test")["args"], "a sample");
    }
}
