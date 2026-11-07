//! Run against a disposable MQTT broker with a simulated EDI service:
//! SUBRAUMCTL_TEST_MQTT_BROKER=tcp://127.0.0.1:21883 \
//!   cargo test --test pizzamat_mqtt -- --ignored

use serde_json::{Value, json};
use std::time::Duration;
use subraumctl::{
    mqtt::{self, MqttCommand, MqttEvent, MqttOptions},
    pizzamat::{Operation, Options, Request, transport},
};
use tokio::sync::mpsc;

async fn execute(mqtt: MqttOptions, options: Options, request: Request) -> eyre::Result<Value> {
    tokio::task::spawn_blocking(move || transport::execute(&mqtt, &options, &request, || false))
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires an explicitly configured disposable MQTT broker"]
async fn bridged_replies_are_isolated_and_tui_uses_its_existing_connection() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let broker = std::env::var("SUBRAUMCTL_TEST_MQTT_BROKER").expect("set SUBRAUMCTL_TEST_MQTT_BROKER to a disposable broker");
        let mqtt = MqttOptions::new(&broker);
        let server = mqtt::connect_once(&MqttOptions::new(&broker)).unwrap();
        let receiver = server.start_consuming();
        server.subscribe_many_same_qos(&["edi.cmd.pizza-list", "edi.cmd.pizza-add", "edi.cmd.pizza-services", "edi.cmd.pizza-lock"], 0).unwrap();
        let fake_edi = tokio::task::spawn_blocking(move || {
            let mut first_lists = Vec::new();
            let mut lists_seen = 0;
            for _ in 0..6 {
                let message = receiver.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
                assert!(!message.retained());
                let request: Value = serde_json::from_slice(message.payload()).unwrap();
                assert_eq!(message.topic(), format!("edi.cmd.{}", request["cmd"].as_str().unwrap()));
                assert!(request["dst"].is_null());
                let destination = request["src"].as_str().unwrap().replace("recv", "send");
                let topic = format!("edi.msg.{destination}");
                let mut replies = Vec::new();
                match request["cmd"].as_str().unwrap() {
                    "pizza-list" => {
                        let body = json!({"user":destination,"msg":format!("Orders for {}", request["user"].as_str().unwrap()),"data":null}).to_string();
                        lists_seen += 1;
                        if lists_seen <= 2 {
                            first_lists.push((topic, body));
                            if lists_seen == 2 { first_lists.reverse(); replies.append(&mut first_lists); }
                        } else { replies.push((topic, body)); }
                    }
                    "pizza-add" => {
                        assert_eq!(request["user"], "alice");
                        assert_eq!(request["args"], "Quattro \"formaggi\"");
                        replies.push((topic, json!({"user":destination,"msg":"Sorry, we have already ordered!","data":null}).to_string()));
                    }
                    "pizza-services" => replies.push((topic, "not JSON".into())),
                    "pizza-lock" => {},
                    other => panic!("unexpected command: {other}"),
                }
                for (topic, body) in replies { mqtt::publish(&server, &topic, &body, false).unwrap(); }
            }
            mqtt::disconnect(&server);
        });
        let options = Options { user: "alice".into(), timeout: Duration::from_secs(2), ..Options::default() };
        let alice = Request::new(Operation::List, "alice", &[]).unwrap();
        let bob = Request::new(Operation::List, "bob", &[]).unwrap();
        let (a, b) = tokio::join!(
            execute(mqtt.clone(), options.clone(), alice.clone()),
            execute(MqttOptions::new(&broker), options.clone(), bob),
        );
        assert_eq!(a.unwrap()["message"], "Orders for alice");
        assert_eq!(b.unwrap()["message"], "Orders for bob");
        let request = Request::new(Operation::Add, "alice", &["Quattro \"formaggi\"".into()]).unwrap();
        assert_eq!(execute(mqtt.clone(), options.clone(), request).await.unwrap()["message"], "Sorry, we have already ordered!");
        let request = Request::new(Operation::Services, "alice", &[]).unwrap();
        assert!(execute(mqtt.clone(), options.clone(), request).await.unwrap_err().to_string().contains("invalid Pizzamat reply JSON"));
        let request = Request::new(Operation::Reset, "alice", &[]).unwrap();
        assert!(execute(mqtt.clone(), options.clone(), request).await.unwrap_err().to_string().contains("timed out"));

        // The TUI transport must use the supervisor, not open a second session.
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, mut event_rx) = mpsc::unbounded_channel();
        mqtt::spawn_supervisor(mqtt, rx, events);
        tx.send(MqttCommand::Connect).unwrap();
        loop {
            match event_rx.recv().await.unwrap() {
                MqttEvent::Connected { .. } => break,
                MqttEvent::ConnectFailed { reason, .. } => panic!("{reason}"),
                _ => {},
            }
        }
        assert_eq!(transport::via_supervisor(&tx, &options, &alice, false).await.unwrap()["message"], "Orders for alice");
        let lock = Request::new(Operation::Lock, "alice", &[]).unwrap();
        let request_tx = tx.clone();
        let pending = tokio::spawn(async move { transport::via_supervisor(&request_tx, &options, &lock, false).await });
        fake_edi.await.unwrap(); // the lock command has been received, but no reply sent
        tx.send(MqttCommand::Disconnect).unwrap();
        assert!(pending.await.unwrap().unwrap_err().to_string().contains("disconnected"));
        loop {
            match event_rx.recv().await.unwrap() {
                MqttEvent::Disconnected { .. } => break,
                MqttEvent::Connected { .. } | MqttEvent::ConnectionLost { .. } => panic!("unexpected replacement MQTT session"),
                _ => {},
            }
        }
        drop(tx);
    }).await.unwrap();
}
