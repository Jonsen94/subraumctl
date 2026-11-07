//! Human and JSON output helpers.
//!
//! Data goes to stdout; tracing logs go to stderr. `--quiet` suppresses human
//! output but not JSON.

use eyre::Result;
use serde_json::json;

use crate::action::Action;

pub fn print_action(action: &Action, json: bool, quiet: bool) -> Result<()> {
    if json {
        let value = match action {
            Action::Publish {
                topic,
                payload,
                retain,
            } => json!({
                "transport": "mqtt",
                "topic": topic,
                "payload": payload,
                "retain": retain,
            }),
            Action::Http { url, body } => json!({
                "transport": "http",
                "url": url,
                "body": body,
            }),
        };
        println!("{}", serde_json::to_string(&value)?);
    } else if !quiet {
        match action {
            Action::Publish { .. } => println!("publish {}", action.describe()),
            Action::Http { .. } => println!("send {}", action.describe()),
        }
    }
    Ok(())
}

pub fn print_published(
    topic: &str,
    payload: &str,
    retain: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "transport": "mqtt",
                "topic": topic,
                "payload": payload,
                "retain": retain,
                "status": "sent",
            }))?
        );
    } else if !quiet {
        println!("published {topic}: {payload}");
    }
    Ok(())
}

pub fn print_http_sent(url: &str, body: &str, json: bool, quiet: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "transport": "http",
                "url": url,
                "body": body,
                "status": "sent",
            }))?
        );
    } else if !quiet {
        println!("sent {} to {url}", body.trim_end());
    }
    Ok(())
}

pub fn print_message(topic: &str, payload: &str, json: bool, quiet: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "topic": topic,
                "payload": payload,
            }))?
        );
    } else if !quiet {
        println!("{topic} {payload}");
    }
    Ok(())
}

pub fn print_decoded(topic: &str, text: &str, json: bool, quiet: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "topic": topic,
                "decoded": text,
            }))?
        );
    } else if !quiet {
        println!("{text}");
    }
    Ok(())
}

pub fn print_info(text: &str, json: bool, quiet: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "info": text,
            }))?
        );
    } else if !quiet {
        println!("{text}");
    }
    Ok(())
}
