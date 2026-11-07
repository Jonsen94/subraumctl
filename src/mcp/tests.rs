use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf};

use super::*;

struct Client {
    reader: BufReader<ReadHalf<DuplexStream>>,
    writer: WriteHalf<DuplexStream>,
}

impl Client {
    async fn send(&mut self, message: Value) {
        self.writer
            .write_all(format!("{message}\n").as_bytes())
            .await
            .unwrap();
    }

    async fn request(&mut self, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .await;
        let mut line = String::new();
        assert_ne!(self.reader.read_line(&mut line).await.unwrap(), 0);
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], 1);
        response
    }

    async fn initialize(&mut self) {
        let response = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "1"}
                }),
            )
            .await;
        assert_eq!(response["result"]["serverInfo"]["name"], "subraumctl");
        assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert!(response["result"]["capabilities"]["resources"].is_null());
        self.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .await;
    }
}

fn start(server: McpServer) -> (Client, tokio::task::JoinHandle<()>) {
    let (client, transport) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server
            .serve(transport)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let (reader, writer) = tokio::io::split(client);
    (
        Client {
            reader: BufReader::new(reader),
            writer,
        },
        task,
    )
}

#[tokio::test]
async fn catalogue_tools_return_structured_data_and_reject_extra_arguments() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut server = McpServer::new(Config::default());
        tools::register(&mut server).unwrap();
        let (mut client, task) = start(server);
        client.initialize().await;
        let cells = client
            .request("tools/call", json!({"name": "white_light_cells"}))
            .await;
        assert_eq!(
            cells["result"]["structuredContent"]["cells"]
                .as_array()
                .unwrap()
                .len(),
            15
        );
        let knobs = client
            .request("tools/call", json!({"name": "knob_list"}))
            .await;
        assert_eq!(
            knobs["result"]["structuredContent"]["knobs"]
                .as_array()
                .unwrap()
                .len(),
            29
        );
        let invalid = client
            .request(
                "tools/call",
                json!({"name": "knob_list", "arguments": {"topic": "#"}}),
            )
            .await;
        assert_eq!(invalid["result"]["isError"], true);
        drop(client);
        task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn foundation_negotiates_lists_no_tools_and_rejects_calls() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (mut client, server) = start(McpServer::new(Config::default()));
        client.initialize().await;
        assert_eq!(
            client.request("tools/list", json!({})).await["result"]["tools"],
            json!([])
        );
        for name in ["publish", "subscribe", "outlet", "smoothiesaur", "unknown"] {
            let response = client
                .request("tools/call", json!({"name": name, "arguments": {}}))
                .await;
            assert_eq!(response["error"]["code"], -32601);
        }
        drop(client);
        server.await.unwrap();
    })
    .await
    .expect("MCP handshake/EOF shutdown timed out");
}

#[tokio::test]
async fn listing_and_calls_enforce_the_same_permissions() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let config: Config = toml::from_str("[tools]\nknob = 'r'\ndmx = 'r'\nalert = 'x'").unwrap();
        let mut server = McpServer::new(config);
        let calls = Arc::new(AtomicUsize::new(0));
        for (name, group, access) in [
            ("knob_get", Group::Knob, Access::Read),
            ("knob_set", Group::Knob, Access::Write),
            ("white_light_status", Group::WhiteLight, Access::Read),
            ("white_light_apply", Group::WhiteLight, Access::Write),
            ("dmx", Group::Dmx, Access::Write),
            ("outlet", Group::Outlet, Access::Write),
            ("publish", Group::Publish, Access::Write),
            ("smoothiesaur_status", Group::Smoothiesaur, Access::Read),
            ("alert_led", Group::Alert, Access::Read),
        ] {
            let calls = calls.clone();
            let definition = Tool::new(
                name,
                "test fixture",
                json!({"type": "object"}).as_object().unwrap().clone(),
            );
            server
                .register(definition, group, access, move |_, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    async { Ok(CallToolResult::success(vec![]).into()) }
                })
                .unwrap();
        }
        let (mut client, task) = start(server);
        client.initialize().await;
        let listed = client.request("tools/list", json!({})).await;
        let names: Vec<_> = listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["knob_get", "white_light_apply", "white_light_status"]
        );
        for name in [
            "knob_set",
            "dmx",
            "outlet",
            "publish",
            "smoothiesaur_status",
            "alert_led",
        ] {
            let response = client
                .request("tools/call", json!({"name": name, "arguments": {}}))
                .await;
            assert_eq!(response["error"]["code"], -32601, "{name}");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        for name in names {
            let response = client
                .request("tools/call", json!({"name": name, "arguments": {}}))
                .await;
            assert!(response.get("error").is_none(), "{response}");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        drop(client);
        task.await.unwrap();
    })
    .await
    .expect("MCP permission test timed out");
}
