//! Pizzamat requests over the existing MQTT/EDI bridge.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use eyre::{Result, ensure, eyre};
use serde_json::{Value, json};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use super::{Options, Request};
use crate::mqtt::{self, MqttCommand, MqttOptions, MqttRequest};

pub fn command_topic(request: &Request, debug: bool) -> String {
    format!("edi.cmd.{}", request.command(debug))
}

pub fn wire_request(options: &Options, request: &Request) -> (MqttRequest, String) {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let id = format!("{}-{time:x}-{sequence:x}", std::process::id());
    let source = format!("subraumctl.recv.{id}");
    let destination = source.replace("recv", "send");
    (
        MqttRequest {
            topic: command_topic(request, options.debug),
            payload: request.payload(options.debug, &source).to_string(),
            reply_topic: format!("edi.msg.{destination}"),
            timeout: options.timeout,
        },
        destination,
    )
}

pub fn preview(options: &Options, request: &Request) -> Value {
    json!({"dry_run": true, "submitted": false, "command": request.command(options.debug),
        "topic": command_topic(request, options.debug), "user": request.user, "args": request.args})
}

fn result(options: &Options, request: &Request, payload: &str, destination: &str) -> Result<Value> {
    let message = decode_reply(payload, destination)?;
    Ok(json!({"dry_run": false, "submitted": true,
        "command": request.command(options.debug), "message": message}))
}

/// One-shot MCP request, using the same broker/options as every other MCP tool.
pub fn execute(
    mqtt: &MqttOptions,
    options: &Options,
    request: &Request,
    cancelled: impl Fn() -> bool,
) -> Result<Value> {
    let (wire, destination) = wire_request(options, request);
    let payload = mqtt::request_once(mqtt, &wire, cancelled).map_err(|error| eyre!(error))?;
    result(options, request, &payload, &destination)
}

/// The TUI shares its supervisor's existing connection and connection controls.
pub async fn via_supervisor(
    mqtt: &UnboundedSender<MqttCommand>,
    options: &Options,
    request: &Request,
    dry_run: bool,
) -> Result<Value> {
    if dry_run {
        return Ok(preview(options, request));
    }
    let (wire, destination) = wire_request(options, request);
    let (reply, response) = oneshot::channel();
    mqtt.send(MqttCommand::Request {
        request: wire,
        reply,
    })
    .map_err(|_| eyre!("MQTT supervisor stopped"))?;
    let payload = response
        .await
        .map_err(|_| eyre!("MQTT supervisor stopped; command may have been applied"))?
        .map_err(|error| eyre!(error))?;
    result(options, request, &payload, &destination)
}

fn decode_reply(payload: &str, destination: &str) -> Result<String> {
    ensure!(payload.len() <= 1024 * 1024, "Pizzamat reply exceeds 1 MiB");
    let value: Value =
        serde_json::from_str(payload).map_err(|_| eyre!("invalid Pizzamat reply JSON"))?;
    ensure!(
        value["user"].as_str() == Some(destination),
        "unexpected Pizzamat reply destination"
    );
    value["msg"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| eyre!("Pizzamat reply has no message"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pizzamat::Operation;

    #[test]
    fn edi_reply_is_scoped_and_only_message_is_returned() {
        let reply = r#"{"msg":"alice: pizza","user":"subraumctl.send.test","data":null}"#;
        assert_eq!(
            decode_reply(reply, "subraumctl.send.test").unwrap(),
            "alice: pizza"
        );
        assert!(decode_reply(reply, "other").is_err());
        assert!(decode_reply(r#"{"user":"other"}"#, "other").is_err());
        assert!(decode_reply("not json", "other").is_err());
    }

    #[test]
    fn bridged_topics_are_unique_and_payloads_keep_identity() {
        let options = Options::default();
        let request = Request::new(Operation::Add, "alice", &["Quattro".into()]).unwrap();
        let (first, destination) = wire_request(&options, &request);
        let (second, _) = wire_request(&options, &request);
        assert_eq!(first.topic, "edi.cmd.pizza-add");
        assert_ne!(first.reply_topic, second.reply_topic);
        let payload: Value = serde_json::from_str(&first.payload).unwrap();
        assert_eq!(payload["user"], "alice");
        assert_eq!(
            payload["src"].as_str().unwrap().replace("recv", "send"),
            destination
        );
        assert_eq!(first.reply_topic, format!("edi.msg.{destination}"));
        assert_eq!(command_topic(&request, true), "edi.cmd.puzza-add");
    }
}
