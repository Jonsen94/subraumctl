#![cfg(feature = "strichliste")]

#[path = "common/strichliste.rs"]
mod fixture;

use serde_json::{Value, json};
use std::{process::Command, time::Duration};
use subraumctl::strichliste::{Client, Options, Request, execute_mcp};

fn options(url: String) -> Options {
    Options {
        url: Some(url),
        user: None,
        user_id: Some(27),
        timeout: Duration::from_secs(2),
    }
}

fn body(request: &str) -> Value {
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[tokio::test]
async fn euro_amounts_keep_fractional_cents_and_use_one_transaction_post() {
    for (request, expected) in [
        (Request::Pay(0.01), -0.01),
        (Request::Add(0.01), 0.01),
        (Request::Pay(1.0), -1.0),
        (Request::Add(1.0), 1.0),
        (Request::Pay(0.001), -0.001),
        (Request::Pay(std::f64::consts::PI), -std::f64::consts::PI),
    ] {
        let (url, server) = fixture::server(vec![(200, fixture::user()), (201, fixture::users())]);
        let result = Client::new(options(url))
            .unwrap()
            .execute(&request, "test")
            .await
            .unwrap();
        assert_eq!(result, json!({"submitted": true, "value_eur": expected}));
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET /strich-api/user/27 "));
        assert!(requests[1].starts_with("POST /strich-api/user/27/transaction "));
        assert_eq!(
            body(&requests[1]),
            json!({"value": expected, "comment": "test"})
        );
    }
}

#[tokio::test]
async fn transfer_matches_the_supplied_contract() {
    let (url, server) = fixture::server(vec![
        (200, fixture::users()),
        (200, fixture::user()),
        (201, json!({})),
    ]);
    let client = Client::new(options(url)).unwrap();
    let users = client.users().await.unwrap();
    client.transfer(&users[1], 0.01, "test").await.unwrap();
    let requests = server.join().unwrap();
    assert!(requests[2].starts_with("POST /strich-api/user/27/transaction "));
    assert_eq!(
        body(&requests[2]),
        json!({
            "value": -0.01, "comment": "to Other: test", "toUserId": 30,
            "toUserComment": "from Me: test",
        })
    );
}

#[tokio::test]
async fn balance_name_resolution_rank_ties_and_mcp_projection() {
    let (url, server) = fixture::server(vec![(200, fixture::user())]);
    let result = execute_mcp(options(url), &Request::Balance, false)
        .await
        .unwrap();
    assert_eq!(result, json!({"balance_eur": 12.345}));
    server.join().unwrap();

    let (url, server) = fixture::server(vec![(200, fixture::users()), (200, fixture::users())]);
    let mut opts = options(url);
    opts.user_id = None;
    opts.user = Some("mE".into());
    let client = Client::new(opts).unwrap();
    assert_eq!(
        client.execute(&Request::Balance, "").await.unwrap(),
        json!({"balance_eur": 12.345})
    );
    assert_eq!(
        client.execute(&Request::Rank, "").await.unwrap(),
        json!({"rank": 2, "weighted_count_of_purchases": 10.5})
    );
    for request in server.join().unwrap() {
        assert!(request.starts_with("GET /strich-api/user "));
    }
}

#[tokio::test]
async fn deployed_balance_is_euros_and_legacy_bare_arrays_still_work() {
    let mut user = fixture::user();
    user["balance"] = json!(-2.4181945088546075);
    let (url, server) = fixture::server(vec![
        (200, user),
        (200, fixture::users()["entries"].clone()),
    ]);
    let client = Client::new(options(url)).unwrap();
    assert_eq!(
        client.execute(&Request::Balance, "").await.unwrap(),
        json!({"balance_eur": -2.4181945088546075})
    );
    assert_eq!(client.users().await.unwrap().len(), 4);
    server.join().unwrap();
}

#[tokio::test]
async fn rank_and_name_lookup_include_later_user_pages() {
    let entries = fixture::users()["entries"].clone();
    let pages = vec![
        (
            200,
            json!({"overallCount": 4, "offset": 0, "limit": 2, "entries": [entries[1], entries[3]]}),
        ),
        (
            200,
            json!({"overallCount": 4, "offset": 2, "limit": 2, "entries": [entries[0], entries[2]]}),
        ),
    ];
    for request in [Request::Balance, Request::Rank] {
        let (url, server) = fixture::server(pages.clone());
        let mut opts = options(url);
        opts.user_id = None;
        opts.user = Some("Me".into());
        let result = Client::new(opts)
            .unwrap()
            .execute(&request, "")
            .await
            .unwrap();
        match request {
            Request::Balance => assert_eq!(result, json!({"balance_eur": 12.345})),
            _ => assert_eq!(
                result,
                json!({"rank": 2, "weighted_count_of_purchases": 10.5})
            ),
        }
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET /strich-api/user "));
        assert!(requests[1].starts_with("GET /strich-api/user?limit=2&offset=2 "));
    }
}

#[tokio::test]
async fn inconsistent_pages_fail_instead_of_returning_partial_rank_or_accounts() {
    let entries = fixture::users()["entries"].clone();
    let first =
        json!({"overallCount": 4, "offset": 0, "limit": 2, "entries": [entries[0], entries[1]]});
    for second in [
        json!({"overallCount": 4, "offset": 2, "limit": 2, "entries": []}),
        json!({"overallCount": 4, "offset": 2, "limit": 2, "entries": [entries[0], entries[2]]}),
        json!({"overallCount": 5, "offset": 2, "limit": 2, "entries": [entries[2], entries[3]]}),
        json!({"overallCount": 4, "offset": 0, "limit": 2, "entries": [entries[2], entries[3]]}),
        json!([entries[2], entries[3]]),
    ] {
        let (url, server) = fixture::server(vec![(200, first.clone()), (200, second)]);
        assert!(
            Client::new(options(url))
                .unwrap()
                .execute(&Request::Rank, "")
                .await
                .is_err()
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }
    for invalid in [
        json!({"entries": []}),
        json!({"overallCount": -1, "entries": []}),
        json!({"overallCount": 100001, "entries": []}),
        json!({"overallCount": 0, "entries": [entries[0]]}),
        json!({"overallCount": 4, "limit": 0, "entries": []}),
        json!({"overallCount": 4, "offset": "PRIVATE", "entries": []}),
    ] {
        let (url, server) = fixture::server(vec![(200, invalid)]);
        let error = Client::new(options(url))
            .unwrap()
            .users()
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("PRIVATE"));
        server.join().unwrap();
    }
}

#[tokio::test]
async fn read_rejections_report_route_and_status_without_response_bodies() {
    for status in [302, 401, 404, 500] {
        let (url, server) = fixture::server(vec![(status, json!({"error": "PRIVATE"}))]);
        let error = Client::new(options(url))
            .unwrap()
            .execute(&Request::Balance, "")
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("GET /user/27"));
        assert!(error.contains(&format!("HTTP {status}")));
        assert!(!error.contains("PRIVATE"));
        assert!(!error.contains("127.0.0.1"));
        server.join().unwrap();
    }
}

#[tokio::test]
async fn mcp_writes_force_the_audit_comment_and_hide_transaction_response() {
    for request in [Request::Pay(1.0), Request::Add(1.0)] {
        let (url, server) = fixture::server(vec![(200, fixture::user()), (201, fixture::users())]);
        let result = execute_mcp(options(url), &request, false).await.unwrap();
        assert_eq!(
            result,
            json!({"submitted": true, "value_eur": request.value().unwrap()})
        );
        let requests = server.join().unwrap();
        assert_eq!(body(&requests[1])["comment"], "triggered by mcp");
        assert_eq!(body(&requests[1]).as_object().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn missing_scores_ambiguous_names_and_mismatched_ids_fail_closed() {
    let mut missing_score = fixture::users();
    missing_score["entries"][1]
        .as_object_mut()
        .unwrap()
        .remove("weightedCountOfPurchases");
    let mut ambiguous = fixture::users();
    ambiguous["entries"][1]["name"] = json!("me");
    for (users, name) in [(missing_score, None), (ambiguous, Some("Me"))] {
        let (url, server) = fixture::server(vec![(200, users)]);
        let mut opts = options(url);
        if let Some(name) = name {
            opts.user_id = None;
            opts.user = Some(name.into());
        }
        let client = Client::new(opts).unwrap();
        let request = if name.is_some() {
            Request::Pay(1.0)
        } else {
            Request::Rank
        };
        assert!(client.execute(&request, "").await.is_err());
        assert_eq!(server.join().unwrap().len(), 1);
    }
    let mut wrong = fixture::user();
    wrong["id"] = json!(30);
    let (url, server) = fixture::server(vec![(200, wrong)]);
    assert!(
        Client::new(options(url))
            .unwrap()
            .execute(&Request::Pay(1.0), "")
            .await
            .is_err()
    );
    server.join().unwrap();
}

#[tokio::test]
async fn null_purchase_scores_count_as_zero_and_malformed_scores_still_fail() {
    let mut users = fixture::users();
    users["entries"][0]["weightedCountOfPurchases"] = Value::Null;
    let (url, server) = fixture::server(vec![(200, users.clone())]);
    let result = execute_mcp(options(url), &Request::Rank, false)
        .await
        .unwrap();
    // Other and Tie have purchases; the configured user ties with Lower at zero.
    assert_eq!(
        result,
        json!({"rank": 3, "weighted_count_of_purchases": 0.0})
    );
    server.join().unwrap();
    for invalid in [json!("NaN"), json!("unknown"), json!(false)] {
        users["entries"][1]["weightedCountOfPurchases"] = invalid;
        let (url, server) = fixture::server(vec![(200, users.clone())]);
        assert!(
            execute_mcp(options(url), &Request::Rank, false)
                .await
                .is_err()
        );
        server.join().unwrap();
    }
}

#[tokio::test]
async fn invalid_amounts_dry_run_and_errors_never_leak_raw_data() {
    for amount in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            execute_mcp(
                options("invalid://unused".into()),
                &Request::Pay(amount),
                true
            )
            .await
            .is_err()
        );
    }
    let preview = execute_mcp(
        options("invalid://unused".into()),
        &Request::Add(0.01),
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        preview,
        json!({"dry_run": true, "submitted": false, "value_eur": 0.01, "comment": "triggered by mcp"})
    );
    for status in [302, 400, 500] {
        let (url, server) = fixture::server(vec![
            (200, fixture::user()),
            (status, json!({"error": "PRIVATE"})),
        ]);
        let error = Client::new(options(url))
            .unwrap()
            .execute(&Request::Add(1.0), "")
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("PRIVATE"));
        assert!(error.to_string().contains("POST /user/27/transaction"));
        assert!(error.to_string().contains(&format!("HTTP {status}")));
        assert_eq!(server.join().unwrap().len(), 2);
    }
}

#[test]
fn cli_bookings_send_euros_and_print_the_same_amount() {
    for (verb, expected) in [("pay", -1.25), ("add", 1.25)] {
        let (url, server) = fixture::server(vec![(200, fixture::user()), (201, json!({}))]);
        let output = Command::new(env!("CARGO_BIN_EXE_subraumctl"))
            .args([
                "--strichliste-url",
                &url,
                "--strichliste-user-id",
                "27",
                "strichliste",
                verb,
                "1.25",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            format!("Booked: {expected} €")
        );
        let requests = server.join().unwrap();
        assert_eq!(
            body(&requests[1]),
            json!({"value": expected, "comment": ""})
        );
    }
}

#[test]
fn cli_live_balance_offline_previews_and_noninteractive_send() {
    let (url, server) = fixture::server(vec![(200, fixture::user())]);
    let output = Command::new(env!("CARGO_BIN_EXE_subraumctl"))
        .args([
            "--strichliste-url",
            &url,
            "--strichliste-user-id",
            "27",
            "--json",
            "strichliste",
            "balance",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"balance_eur": 12.345})
    );
    server.join().unwrap();
    for verb in ["pay", "add", "send"] {
        let output = Command::new(env!("CARGO_BIN_EXE_subraumctl"))
            .args(["--dry-run", "--json", "strichliste", verb, "0.01"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["submitted"], false);
        assert_eq!(value["value_eur"], if verb == "add" { 0.01 } else { -0.01 });
    }
    let output = Command::new(env!("CARGO_BIN_EXE_subraumctl"))
        .args(["strichliste", "send", "1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("interactive terminal"));
}
