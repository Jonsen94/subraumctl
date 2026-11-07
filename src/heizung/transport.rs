//! Private EDI reply queues, following the Pizzamat AMQP integration.

use eyre::{Result, ensure, eyre};
use futures_lite::StreamExt;
use lapin::{BasicProperties, Connection, ConnectionProperties, options::*, types::FieldTable};
use serde_json::{Value, json};

use super::{Options, Request};

pub async fn execute(options: &Options, request: &Request, dry_run: bool) -> Result<Value> {
    if dry_run {
        return Ok(
            json!({"dry_run": true, "submitted": false, "command": "heizung", "args": request.args()}),
        );
    }
    // Addresses may contain credentials; never propagate transport errors.
    let connection = tokio::time::timeout(
        options.timeout,
        Connection::connect(&options.amqp_url, ConnectionProperties::default()),
    )
    .await
    .map_err(|_| eyre!("heizung connection timed out"))?
    .map_err(|_| eyre!("heizung AMQP connection failed"))?;
    let result = tokio::time::timeout(options.timeout, exchange(&connection, request))
        .await.map_err(|_| eyre!("heizung reply timed out; command may have been applied; check status before retrying"))
        .and_then(|result| result);
    // Also close on malformed replies, unrouted commands and timeouts.
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        connection.close(200, "done"),
    )
    .await;
    result.map(|mut value| {
        value["dry_run"] = json!(false);
        value["submitted"] = json!(true);
        value
    })
}

async fn exchange(connection: &Connection, request: &Request) -> Result<Value> {
    let channel = connection
        .create_channel()
        .await
        .map_err(|_| eyre!("heizung AMQP channel failed"))?;
    let queue = channel
        .queue_declare(
            "",
            QueueDeclareOptions {
                exclusive: true,
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(|_| eyre!("heizung reply queue failed"))?;
    let source = format!("subraumctl.recv.{}", queue.name());
    let destination = source.replace("recv", "send");
    channel
        .queue_bind(
            queue.name().as_str(),
            "msg",
            &destination,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|_| eyre!("heizung reply binding failed (EDI msg exchange required)"))?;
    let mut consumer = channel
        .basic_consume(
            queue.name().as_str(),
            "",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(|_| eyre!("heizung reply consumer failed"))?;
    channel
        .confirm_select(ConfirmSelectOptions::default())
        .await
        .map_err(|_| eyre!("heizung publisher confirmation setup failed"))?;
    let payload = serde_json::to_vec(&request.payload(&source))?;
    let confirmation = channel
        .basic_publish(
            "cmd",
            "heizung",
            BasicPublishOptions {
                mandatory: true,
                ..Default::default()
            },
            &payload,
            BasicProperties::default().with_content_type("application/json".into()),
        )
        .await
        .map_err(|_| eyre!("heizung publish failed; command may have been applied"))?
        .await
        .map_err(|_| eyre!("heizung publish confirmation failed; command may have been applied"))?;
    ensure!(
        confirmation.is_ack() && confirmation.take_message().is_none(),
        "heizung command was not routed or acknowledged (check service)"
    );
    let delivery = consumer
        .next()
        .await
        .ok_or_else(|| eyre!("heizung reply consumer closed; command may have been applied"))?
        .map_err(|_| eyre!("heizung reply failed; command may have been applied"))?;
    decode_reply(&delivery.data, &destination, request)
}

fn decode_reply(bytes: &[u8], destination: &str, request: &Request) -> Result<Value> {
    ensure!(bytes.len() <= 4096, "heizung reply exceeds 4096 bytes");
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| eyre!("invalid heizung reply JSON"))?;
    ensure!(
        value["user"].as_str() == Some(destination),
        "unexpected heizung reply destination"
    );
    let message = value["msg"]
        .as_str()
        .ok_or_else(|| eyre!("heizung reply has no message"))?;
    request.decode(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_are_bounded_scoped_and_redacted() {
        let request = Request::get(vec![super::super::Room::Hack]).unwrap();
        let reply = json!({"user": "subraumctl.send.test", "msg": "hack: 21.0, 00:01:02 remaining", "data": "PRIVATE"}).to_string();
        let decoded = decode_reply(reply.as_bytes(), "subraumctl.send.test", &request).unwrap();
        assert_eq!(decoded["rooms"][0]["remaining_seconds"], 62);
        assert!(!decoded.to_string().contains("PRIVATE"));
        assert!(decode_reply(reply.as_bytes(), "other", &request).is_err());
        for reply in [b"not JSON".as_slice(), b"{}", &[b' '; 4097]] {
            assert!(decode_reply(reply, "other", &request).is_err());
        }
    }
}
