//! Private EDI reply routing, with mandatory, confirmed AMQP publications.

use eyre::{Result, ensure, eyre};
use futures_lite::StreamExt;
use lapin::{BasicProperties, Connection, ConnectionProperties, options::*, types::FieldTable};
use serde_json::Value;

use super::{MAX_REPLY_BYTES, MAX_RESULTS, Options, Request, Response, validate_text};

/// Execute once, without retries. Dry runs perform no I/O, even with invalid options.
/// Search waits for one bounded EDI JSON `msg` string; Play waits only for a
/// mandatory publisher confirmation. Errors never include backend text or URLs.
pub async fn execute(options: &Options, request: &Request, dry_run: bool) -> Result<Response> {
    if dry_run {
        return Ok(Response::DryRun {
            command: request.command(),
            args: request.args().to_owned(),
        });
    }
    ensure!(
        !options.timeout.is_zero(),
        "soundboard timeout must be positive"
    );
    let deadline = tokio::time::Instant::now()
        .checked_add(options.timeout)
        .ok_or_else(|| eyre!("soundboard timeout is too large"))?;
    let connection = tokio::time::timeout_at(
        deadline,
        Connection::connect(&options.amqp_url, ConnectionProperties::default()),
    )
    .await
    .map_err(|_| eyre!("soundboard connection timed out; command was not published"))?
    .map_err(|_| eyre!("soundboard AMQP connection failed; command was not published"))?;
    let result = tokio::time::timeout_at(deadline, exchange(&connection, request))
        .await
        .map_err(|_| {
            eyre!("soundboard request timed out; execution status is unknown; do not retry blindly")
        })
        .and_then(|result| result);
    // Always close, including on timeout or malformed replies, to remove the
    // exclusive queue. Cleanup failures cannot invalidate a confirmed publish.
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        connection.close(200, "done"),
    )
    .await;
    result
}

async fn exchange(connection: &Connection, request: &Request) -> Result<Response> {
    let channel = connection
        .create_channel()
        .await
        .map_err(|_| eyre!("soundboard AMQP channel failed; command was not published"))?;
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
        .map_err(|_| eyre!("soundboard reply queue failed; command was not published"))?;
    // Server-generated names are unique across clients. Replace only our recv
    // component, never a coincidental substring in the generated queue name.
    let source = format!("subraumctl.recv.{}", queue.name());
    let destination = format!("subraumctl.send.{}", queue.name());
    channel
        .queue_bind(
            queue.name().as_str(),
            "msg",
            &destination,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|_| eyre!("soundboard reply binding failed; command was not published"))?;
    // Install the consumer before publishing Search so fast replies are safe.
    // Play deliberately has no reply requirement.
    let mut consumer = if request.read_only() {
        Some(
            channel
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
                .map_err(|_| {
                    eyre!("soundboard reply consumer failed; command was not published")
                })?,
        )
    } else {
        None
    };
    channel
        .confirm_select(ConfirmSelectOptions::default())
        .await
        .map_err(|_| {
            eyre!("soundboard publisher confirmation setup failed; command was not published")
        })?;
    let payload = serde_json::to_vec(&request.payload(&source))
        .map_err(|_| eyre!("soundboard request encoding failed; command was not published"))?;
    let confirmation = channel
        .basic_publish(
            "cmd",
            request.command(),
            BasicPublishOptions {
                mandatory: true,
                ..Default::default()
            },
            &payload,
            BasicProperties::default().with_content_type("application/json".into()),
        )
        .await
        .map_err(|_| eyre!("soundboard publish failed; execution status is unknown; do not retry blindly"))?
        .await
        .map_err(|_| eyre!("soundboard publish confirmation failed; execution status is unknown; do not retry blindly"))?;
    ensure!(
        confirmation.is_ack() && confirmation.take_message().is_none(),
        "soundboard command was not routed or acknowledged; execution is not confirmed; do not retry blindly"
    );
    let Some(ref mut consumer) = consumer else {
        return Ok(Response::PlayPublished);
    };
    let delivery = consumer
        .next()
        .await
        .ok_or_else(|| {
            eyre!("soundboard reply consumer closed after publication; no automatic retry")
        })?
        .map_err(|_| eyre!("soundboard reply failed after publication; no automatic retry"))?;
    decode_reply(
        &delivery.data,
        delivery.exchange.as_str(),
        delivery.routing_key.as_str(),
        &destination,
    )
}

fn decode_reply(bytes: &[u8], exchange: &str, route: &str, destination: &str) -> Result<Response> {
    ensure!(
        bytes.len() <= MAX_REPLY_BYTES,
        "soundboard reply exceeds 64 KiB"
    );
    // The private queue's AMQP route correlates the reply. Python EDI may put
    // the caller identity in JSON user; it need not equal the destination.
    ensure!(
        exchange == "msg" && route == destination,
        "unexpected soundboard reply destination"
    );
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| eyre!("invalid soundboard reply JSON"))?;
    let message = value["msg"]
        .as_str()
        .ok_or_else(|| eyre!("soundboard reply has no message string"))?;
    let mut matches = Vec::new();
    let mut truncated = false;
    for (index, line) in message
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        // sobfind prefixes its results with a summary, not a sample name.
        if index == 0
            && line
                .trim()
                .strip_prefix("Matched ")
                .and_then(|text| text.strip_suffix(" files:"))
                .is_some_and(|count| {
                    !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit())
                })
        {
            continue;
        }
        if matches.len() == MAX_RESULTS {
            truncated = true;
            break;
        }
        // Keep literal names (including surrounding spaces) playable through
        // Request::play, and reject terminal controls or oversized names.
        validate_text(line).map_err(|_| eyre!("invalid soundboard sample in reply"))?;
        matches.push(line.to_owned());
    }
    Ok(Response::Search { matches, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DESTINATION: &str = "subraumctl.send.test";

    fn decode_message(message: &str) -> Result<Response> {
        let bytes = json!({"user": "subraumctl", "msg": message, "data": "PRIVATE"}).to_string();
        decode_reply(bytes.as_bytes(), "msg", DESTINATION, DESTINATION)
    }

    #[test]
    fn matches_are_literal_lines_and_empty_lines_are_skipped() {
        assert_eq!(
            decode_message("\n a  sample \r\n\n\r\nother\n  \n").unwrap(),
            Response::Search {
                matches: vec![" a  sample ".into(), "other".into()],
                truncated: false,
            }
        );
        assert_eq!(
            decode_message("").unwrap(),
            Response::Search {
                matches: vec![],
                truncated: false,
            }
        );
    }

    #[test]
    fn search_summary_is_not_a_playable_sample() {
        for message in [
            "Matched 2 files:\nfirst\nsecond\n",
            "\r\nMatched 2 files:\r\nfirst\r\nsecond\r\n",
        ] {
            assert_eq!(
                decode_message(message).unwrap(),
                Response::Search {
                    matches: vec!["first".into(), "second".into()],
                    truncated: false,
                }
            );
        }
        assert_eq!(
            decode_message("Matched 0 files:\n").unwrap(),
            Response::Search {
                matches: vec![],
                truncated: false,
            }
        );
        // Only the leading numeric summary is special; preserve literal names.
        assert_eq!(
            decode_message("Matched n files:\nMatched 1 files:").unwrap(),
            Response::Search {
                matches: vec!["Matched n files:".into(), "Matched 1 files:".into()],
                truncated: false,
            }
        );
        let message = format!(
            "Matched {MAX_RESULTS} files:\n{}",
            "sample\n".repeat(MAX_RESULTS)
        );
        let Response::Search { matches, truncated } = decode_message(&message).unwrap() else {
            panic!("expected search result");
        };
        assert_eq!(matches.len(), MAX_RESULTS);
        assert!(!truncated);
    }

    #[test]
    fn reply_route_not_json_user_correlates_the_request() {
        for user in ["subraumctl", DESTINATION, "other"] {
            let bytes = json!({"user": user, "msg": "sample"}).to_string();
            assert!(decode_reply(bytes.as_bytes(), "msg", DESTINATION, DESTINATION).is_ok());
            assert!(decode_reply(bytes.as_bytes(), "msg", "other", DESTINATION).is_err());
            assert!(decode_reply(bytes.as_bytes(), "other", DESTINATION, DESTINATION).is_err());
        }
    }

    #[test]
    fn replies_are_bounded_and_extra_matches_are_visible() {
        for count in [MAX_RESULTS - 1, MAX_RESULTS, MAX_RESULTS + 1] {
            let message = "sample\n".repeat(count) + "\n  \n";
            let Response::Search { matches, truncated } = decode_message(&message).unwrap() else {
                panic!("expected search result");
            };
            assert_eq!(matches.len(), count.min(MAX_RESULTS));
            assert_eq!(truncated, count > MAX_RESULTS);
        }
        assert!(decode_message(&"x".repeat(super::super::MAX_ARGUMENT_BYTES)).is_ok());
        assert!(decode_message(&"x".repeat(super::super::MAX_ARGUMENT_BYTES + 1)).is_err());
        for message in ["a\tb", "a\0b", "a\u{1b}[31m"] {
            assert!(decode_message(message).is_err());
        }
        for bytes in [
            b"not JSON".as_slice(),
            b"{}",
            b"{\"msg\":null}",
            b"{\"msg\":[]}",
            b"\xff",
        ] {
            assert!(decode_reply(bytes, "msg", DESTINATION, DESTINATION).is_err());
        }
        // Whitespace makes otherwise valid JSON exactly the wire-size limit.
        let mut bytes = br#"{"msg":""}"#.to_vec();
        bytes.resize(MAX_REPLY_BYTES, b' ');
        assert!(decode_reply(&bytes, "msg", DESTINATION, DESTINATION).is_ok());
        bytes.push(b' ');
        assert!(decode_reply(&bytes, "msg", DESTINATION, DESTINATION).is_err());
        let error = decode_reply(b"PRIVATE", "msg", DESTINATION, DESTINATION).unwrap_err();
        assert!(!format!("{error:?}").contains("PRIVATE"));
    }

    #[tokio::test]
    async fn dry_runs_do_not_connect_or_validate_connection_options() {
        let options = Options {
            amqp_url: "invalid://PRIVATE:SECRET@invalid".into(),
            timeout: std::time::Duration::ZERO,
        };
        for request in [
            Request::search("a query").unwrap(),
            Request::play(" a  sample ").unwrap(),
        ] {
            let response = execute(&options, &request, true).await.unwrap();
            assert_eq!(
                response,
                Response::DryRun {
                    command: request.command(),
                    args: request.args().into(),
                }
            );
            assert!(!format!("{response:?}").contains("SECRET"));
        }
    }
}
