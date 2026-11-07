//! Bounded request/reply operations on MQTT. Subscribe before publishing;
//! ignore retained replies and never replay requests after reconnecting.

use super::{MqttOptions, connect_once, disconnect, paho, publish, subscribe};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

const TIMEOUT: &str =
    "MQTT reply timed out; command may have been applied; check orders before retrying";

#[derive(Debug)]
pub struct MqttRequest {
    pub topic: String,
    pub payload: String,
    pub reply_topic: String,
    pub timeout: Duration,
}

pub(super) struct Pending {
    request: MqttRequest,
    deadline: Instant,
    reply: oneshot::Sender<Result<String, String>>,
}

impl Pending {
    pub(super) fn new(
        request: MqttRequest,
        reply: oneshot::Sender<Result<String, String>>,
    ) -> Self {
        Self {
            deadline: Instant::now() + request.timeout,
            request,
            reply,
        }
    }

    fn finish(self, client: Option<&paho::Client>, result: Result<String, String>) {
        if let Some(client) = client.filter(|client| client.is_connected()) {
            let _ = client.unsubscribe(&self.request.reply_topic);
        }
        let _ = self.reply.send(result);
    }
}

pub(super) fn start(client: &paho::Client, request: &MqttRequest) -> Result<(), String> {
    subscribe(client, &request.reply_topic)
        .map_err(|_| "MQTT reply subscription failed".to_string())?;
    if publish(client, &request.topic, &request.payload, false).is_err() {
        let _ = client.unsubscribe(&request.reply_topic);
        return Err("MQTT publish failed; command may have been applied".into());
    }
    Ok(())
}

fn payload(message: &paho::Message) -> Result<String, String> {
    if message.payload().len() > 1024 * 1024 {
        return Err("MQTT reply exceeds 1 MiB".into());
    }
    std::str::from_utf8(message.payload())
        .map(str::to_string)
        .map_err(|_| "MQTT reply is not UTF-8".into())
}

pub(super) fn handle_reply(
    pending: &mut Vec<Pending>,
    client: Option<&paho::Client>,
    message: &paho::Message,
) {
    if message.retained() {
        return;
    }
    if let Some(index) = pending
        .iter()
        .position(|item| item.request.reply_topic == message.topic())
    {
        pending.remove(index).finish(client, payload(message));
    }
}

pub(super) fn expire_pending(pending: &mut Vec<Pending>, client: Option<&paho::Client>) {
    let mut index = 0;
    while index < pending.len() {
        if pending[index].reply.is_closed() || Instant::now() >= pending[index].deadline {
            pending.remove(index).finish(client, Err(TIMEOUT.into()));
        } else {
            index += 1;
        }
    }
}

pub(super) fn fail_pending(
    pending: &mut Vec<Pending>,
    client: Option<&paho::Client>,
    reason: &str,
) {
    for request in pending.drain(..) {
        request.finish(client, Err(reason.into()));
    }
}

pub fn request_once(
    options: &MqttOptions,
    request: &MqttRequest,
    cancelled: impl Fn() -> bool,
) -> Result<String, String> {
    if cancelled() {
        return Err("request cancelled".into());
    }
    let mut client = connect_once(options).map_err(|_| "MQTT connection failed".to_string())?;
    client.set_timeout(Duration::from_secs(5));
    let receiver = client.start_consuming();
    let result = (|| {
        if cancelled() {
            return Err("request cancelled".into());
        }
        start(&client, request)?;
        let deadline = Instant::now() + request.timeout;
        loop {
            if cancelled() {
                return Err("request cancelled; command may have been applied".into());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(TIMEOUT.into());
            }
            match receiver.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(Some(message))
                    if message.topic() == request.reply_topic && !message.retained() =>
                {
                    return payload(&message);
                }
                Ok(None) => {
                    return Err("MQTT connection closed; command may have been applied".into());
                }
                Err(_) if !client.is_connected() => {
                    return Err("MQTT connection lost; command may have been applied".into());
                }
                _ => {}
            }
        }
    })();
    let _ = client.unsubscribe(&request.reply_topic);
    disconnect(&client);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(
        topic: &str,
        timeout: Duration,
    ) -> (Pending, oneshot::Receiver<Result<String, String>>) {
        let (tx, rx) = oneshot::channel();
        (
            Pending::new(
                MqttRequest {
                    topic: "command".into(),
                    payload: "{}".into(),
                    reply_topic: topic.into(),
                    timeout,
                },
                tx,
            ),
            rx,
        )
    }

    #[test]
    fn replies_are_scoped_and_retained_messages_are_ignored() {
        let (first, mut first_rx) = pending("reply/first", Duration::from_secs(1));
        let (second, mut second_rx) = pending("reply/second", Duration::from_secs(1));
        let mut requests = vec![first, second];
        handle_reply(
            &mut requests,
            None,
            &paho::Message::new_retained("reply/first", "stale", 0),
        );
        handle_reply(
            &mut requests,
            None,
            &paho::Message::new("other", "unrelated", 0),
        );
        assert!(first_rx.try_recv().is_err());
        handle_reply(
            &mut requests,
            None,
            &paho::Message::new("reply/second", "second", 0),
        );
        assert_eq!(second_rx.try_recv().unwrap().unwrap(), "second");
        assert_eq!(requests.len(), 1);
        fail_pending(&mut requests, None, "disconnected");
        assert_eq!(first_rx.try_recv().unwrap().unwrap_err(), "disconnected");
        assert!(requests.is_empty());
    }

    #[test]
    fn timed_out_and_cancelled_requests_are_removed() {
        let (expired, mut rx) = pending("expired", Duration::ZERO);
        let (cancelled, abandoned) = pending("cancelled", Duration::from_secs(1));
        drop(abandoned);
        let mut requests = vec![expired, cancelled];
        expire_pending(&mut requests, None);
        assert!(requests.is_empty());
        assert!(
            rx.try_recv()
                .unwrap()
                .unwrap_err()
                .contains("may have been applied")
        );
    }
}
