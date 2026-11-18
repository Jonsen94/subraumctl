use super::*;
use crate::{
    cli::{GraphLayout, TuiView},
    color::{Theme, ThemePreset},
    mqtt::{MqttCommand, MqttOptions},
    tui::TuiOptions,
};
use crossterm::event::KeyModifiers;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::strichliste::test_fixture as fixture;

fn app() -> (
    App,
    UnboundedReceiver<Command>,
    UnboundedReceiver<MqttCommand>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (mqtt_tx, mqtt_rx) = mpsc::unbounded_channel();
    let mut app = App::new(
        TuiOptions {
            mqtt: MqttOptions::new("tcp://unused:1883"),
            view: TuiView::Strichliste,
            connect: false,
            safe_mode: true,
            theme: Theme::preset(ThemePreset::Dark),
            theme_preset: ThemePreset::Dark,
            smoothie_url: String::new(),
            pizzamat: crate::pizzamat::Options::default(),
            heizung: crate::heizung::Options {
                amqp_url: String::new(),
                timeout: Duration::from_secs(1),
            },
            music: crate::music::Options::default(),
            soundboard: crate::soundboard::Options::default(),
            strichliste: Options::default(),
            influx: crate::influx::Options::default(),
            graph_minutes: 60,
            graph_layout: GraphLayout::Table,
            verbose: 0,
            dry_run: false,
        },
        mqtt_tx,
        mpsc::unbounded_channel().0,
        mpsc::unbounded_channel().0,
        mpsc::unbounded_channel().0,
        mpsc::unbounded_channel().0,
        mpsc::unbounded_channel().0,
    );
    app.strichliste.tx = Some(tx);
    (app, rx, mqtt_rx)
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

fn book_form(app: &mut App, focus: usize, amount: &str, comment: &str) {
    app.strichliste.focus = focus;
    press(app, KeyCode::Enter);
    type_text(app, amount);
    press(app, KeyCode::Enter);
    type_text(app, comment);
    press(app, KeyCode::Enter);
}

fn screen(app: &mut App) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| crate::tui::ui::draw(frame, app))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn options(url: String) -> Options {
    Options {
        url: Some(url),
        user_id: Some(27),
        user: None,
        timeout: Duration::from_secs(2),
    }
}

#[test]
fn payment_forms_own_text_keys_validate_amounts_and_obey_safe_mode() {
    for (focus, pay) in [(0, true), (1, false)] {
        let (mut app, mut rx, mut mqtt) = app();
        book_form(&mut app, focus, "0.001", "qQcdr?S[]");
        assert!(!app.should_quit);
        assert!(app.safe_mode);
        assert!(mqtt.try_recv().is_err());
        assert!(matches!(
            app.popup,
            Some(Popup::ConfirmCommand {
                selected: ConfirmChoice::Cancel,
                ..
            })
        ));
        assert!(rx.try_recv().is_err());
        press(&mut app, KeyCode::Enter); // Cancel is the default.
        assert!(rx.try_recv().is_err());

        book_form(&mut app, focus, "0.001", "qQcdr?S[]");
        press(&mut app, KeyCode::Char('y'));
        let Command::Book { request, comment } = rx.try_recv().unwrap() else {
            panic!("booking expected")
        };
        assert_eq!(request.value().unwrap(), Some(if pay { -0.1 } else { 0.1 }));
        assert_eq!(comment, "qQcdr?S[]");
        assert!(app.strichliste.busy);
        press(&mut app, KeyCode::Enter);
        assert!(rx.try_recv().is_err());
    }
    for amount in ["0", "-1", "NaN", "inf"] {
        let (mut app, mut rx, _) = app();
        app.strichliste.focus = 0;
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, amount);
        press(&mut app, KeyCode::Enter);
        assert!(app.strichliste.error.is_some());
        assert!(matches!(
            app.strichliste.form.as_ref().unwrap().step,
            Step::Amount
        ));
        assert!(rx.try_recv().is_err());
        press(&mut app, KeyCode::Esc);
        assert!(app.strichliste.form.is_none());
    }
    let (mut app, mut rx, _) = app();
    app.safe_mode = false;
    book_form(&mut app, 0, "1", "");
    assert!(matches!(
        rx.try_recv().unwrap(),
        Command::Book {
            request: Request::Pay(1.0),
            ..
        }
    ));
    assert!(app.popup.is_none());
}

#[tokio::test]
async fn recipient_search_excludes_self_then_transfers_once_with_the_supplied_contract() {
    let (mut app, mut rx, _) = app();
    app.strichliste.focus = 2;
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "o");
    press(&mut app, KeyCode::Enter);
    let command = rx.try_recv().unwrap();
    let (url, http) = fixture::server(vec![(200, fixture::user()), (200, fixture::users())]);
    let result = execute(options(url), &command, false).await.unwrap();
    app.handle_strichliste_event(Event {
        command,
        result: Ok(result),
    });
    let form = app.strichliste.form.as_ref().unwrap();
    assert!(form.recipients.iter().all(|user| user.id != 27));
    // Alphabetic results: Lower, Other. Navigate to the intended account.
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.strichliste
            .form
            .as_ref()
            .unwrap()
            .recipient
            .as_ref()
            .unwrap()
            .id,
        30
    );
    type_text(&mut app, "0.01");
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "test");
    press(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Send 0.01 EUR to Other"));
    assert!(rx.try_recv().is_err());
    press(&mut app, KeyCode::Char('y'));
    let command = rx.try_recv().unwrap();
    let (url, http_write) = fixture::server(vec![(200, fixture::user()), (201, json!({}))]);
    assert!(matches!(
        execute(options(url), &command, false).await.unwrap(),
        Response::Submitted
    ));
    let requests = http_write.join().unwrap();
    let body: Value = serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body,
        json!({
            "value": -1.0, "comment": "to Other: test", "toUserId": 30,
            "toUserComment": "from Me: test",
        })
    );
    http.join().unwrap();
}

#[test]
fn cancelled_search_does_not_reopen_the_form_or_send_and_global_navigation_recovers() {
    let (mut app, mut rx, _) = app();
    app.strichliste.focus = 2;
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "qQcdr?S[]");
    assert!(!app.should_quit);
    assert!(rx.try_recv().is_err());
    press(&mut app, KeyCode::Enter);
    let command = rx.try_recv().unwrap();
    press(&mut app, KeyCode::Esc);
    app.handle_strichliste_event(Event {
        command,
        result: Ok(Response::Recipients(vec![])),
    });
    assert!(app.strichliste.form.is_none());
    app.on_tick();
    assert!(rx.try_recv().is_err());
    press(&mut app, KeyCode::Char(']'));
    assert_eq!(app.section, Section::Overview);
    press(&mut app, KeyCode::Char('['));
    assert_eq!(app.section, Section::Strichliste);
    press(&mut app, KeyCode::Enter);
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(app.should_quit);
}

#[test]
fn reads_respect_no_connect_refresh_after_submission_and_preserve_stale_values_on_error() {
    let (mut app, mut rx, mut mqtt) = app();
    app.maybe_refresh_strichliste();
    assert!(rx.try_recv().is_err());
    press(&mut app, KeyCode::Char('r'));
    assert!(matches!(rx.try_recv().unwrap(), Command::Refresh));
    assert!(mqtt.try_recv().is_err());
    app.handle_strichliste_event(Event {
        command: Command::Refresh,
        result: Ok(Response::Account {
            balance: Ok(12.345),
            rank: Ok((2, 10.5)),
        }),
    });
    let text = screen(&mut app);
    for expected in [
        "Balance: 12.35 EUR",
        "Rank: 2",
        "weighted purchases: 10.5",
        "Add credit",
        "Send to another user",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    app.options.connect = true;
    app.maybe_refresh_strichliste();
    app.on_tick();
    assert!(rx.try_recv().is_err()); // No repeat before the refresh interval.
    app.handle_strichliste_event(Event {
        command: Command::Book {
            request: Request::Pay(0.01),
            comment: String::new(),
        },
        result: Ok(Response::Submitted),
    });
    assert!(matches!(rx.try_recv().unwrap(), Command::Refresh));
    app.handle_strichliste_event(Event {
        command: Command::Refresh,
        result: Ok(Response::Account {
            balance: Err("offline".into()),
            rank: Err("offline".into()),
        }),
    });
    assert_eq!(app.strichliste.balance, Some(12.345));
    assert_eq!(app.strichliste.rank, Some((2, 10.5)));
    assert!(app.strichliste.balance_stale && app.strichliste.rank_stale);
    assert!(screen(&mut app).contains("(previous)"));
    app.handle_strichliste_event(Event {
        command: Command::Book {
            request: Request::Pay(0.01),
            comment: String::new(),
        },
        result: Err("outcome unknown".into()),
    });
    app.on_tick();
    assert!(rx.try_recv().is_err());
    assert!(screen(&mut app).contains("outcome unknown"));
    assert_eq!(Section::Strichliste.bit(), 0x10000);
}

#[tokio::test]
async fn background_worker_reads_balance_rank_and_dry_run_never_connects() {
    let (url, http) = fixture::server(vec![(200, fixture::user()), (200, fixture::users())]);
    let (tx, rx) = mpsc::unbounded_channel();
    let (events, mut replies) = mpsc::unbounded_channel();
    spawn_worker(options(url), false, rx, events);
    tx.send(Command::Refresh).unwrap();
    drop(tx);
    let event = tokio::time::timeout(Duration::from_secs(3), replies.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        event.result,
        Ok(Response::Account {
            balance: Ok(12.345),
            rank: Ok((2, 10.5))
        })
    ));
    assert!(replies.recv().await.is_none());
    http.join().unwrap();

    let (tx, rx) = mpsc::unbounded_channel();
    let (events, mut replies) = mpsc::unbounded_channel();
    spawn_worker(
        options("invalid://PRIVATE:SECRET@unused".into()),
        true,
        rx,
        events,
    );
    for command in [
        Command::Refresh,
        Command::Search("someone".into()),
        Command::Book {
            request: Request::Pay(0.01),
            comment: "TUI".into(),
        },
        Command::Book {
            request: Request::Add(1.0),
            comment: "TUI".into(),
        },
    ] {
        tx.send(command).unwrap();
    }
    drop(tx);
    for _ in 0..4 {
        let event = tokio::time::timeout(Duration::from_secs(1), replies.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event.result, Ok(Response::DryRun)));
        assert!(!event.command.preview_lines().join(" ").contains("SECRET"));
    }
    assert!(replies.recv().await.is_none());
}

#[test]
fn auto_refresh_runs_every_ten_seconds_only_when_the_tab_is_idle_and_visible() {
    let (mut app, mut rx, mut mqtt) = app();
    app.options.connect = true;
    app.on_tick();
    assert!(matches!(rx.try_recv().unwrap(), Command::Refresh));
    app.on_tick();
    press(&mut app, KeyCode::Char('r'));
    assert!(rx.try_recv().is_err()); // An in-flight read is never queued twice.
    app.handle_strichliste_event(Event {
        command: Command::Refresh,
        result: Ok(Response::Account {
            balance: Ok(1.0),
            rank: Ok((1, 1.0)),
        }),
    });
    app.strichliste.last_refresh = Some(Instant::now() - Duration::from_secs(9));
    app.on_tick();
    assert!(rx.try_recv().is_err());
    app.strichliste.last_refresh = Some(Instant::now() - Duration::from_secs(11));
    app.section = Section::Overview;
    app.on_tick();
    assert!(rx.try_recv().is_err());
    app.section = Section::Strichliste;
    press(&mut app, KeyCode::Enter);
    app.on_tick();
    assert!(rx.try_recv().is_err()); // Editing takes priority over automatic reads.
    press(&mut app, KeyCode::Esc);
    app.popup = Some(Popup::Quit {
        selected: ConfirmChoice::Cancel,
    });
    app.on_tick();
    assert!(rx.try_recv().is_err());
    press(&mut app, KeyCode::Esc);
    app.on_tick();
    assert!(matches!(rx.try_recv().unwrap(), Command::Refresh));
    app.handle_strichliste_event(Event {
        command: Command::Refresh,
        result: Err("offline".into()),
    });
    app.on_tick();
    assert!(rx.try_recv().is_err()); // Errors are also throttled.
    press(&mut app, KeyCode::Char('r'));
    assert!(matches!(rx.try_recv().unwrap(), Command::Refresh)); // Explicit refresh bypasses timer.
    assert!(mqtt.try_recv().is_err()); // r did not reconnect MQTT.
}

#[test]
fn balance_is_rounded_by_default_and_precision_toggle_only_changes_presentation() {
    let (mut app, mut rx, mut mqtt) = app();
    let value = -2.4181945088546075;
    app.strichliste.balance = Some(value);
    let text = screen(&mut app);
    assert!(text.contains("Balance: -2.42 EUR"));
    assert!(text.contains("rounded to cents"));
    assert!(!text.contains("Refresh balance and rank"));
    assert!(text.contains("r refresh"));
    press(&mut app, KeyCode::Char('p'));
    let text = screen(&mut app);
    assert!(text.contains("Balance: -2.4181945088546075 EUR"));
    assert!(text.contains("full precision"));
    assert_eq!(app.strichliste.balance, Some(value));
    press(&mut app, KeyCode::Char('p'));
    assert!(screen(&mut app).contains("Balance: -2.42 EUR"));
    app.strichliste.balance = Some(-0.001);
    assert!(screen(&mut app).contains("Balance: 0.00 EUR"));
    assert!(rx.try_recv().is_err());
    assert!(mqtt.try_recv().is_err());

    press(&mut app, KeyCode::Enter); // Pay is now the first action.
    type_text(&mut app, "0.01");
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "pr");
    assert_eq!(app.strichliste.form.as_ref().unwrap().input.value(), "pr");
    assert!(!app.strichliste.full_precision);
    assert!(rx.try_recv().is_err());
    assert!(mqtt.try_recv().is_err());
}

#[cfg(feature = "tui-mouse")]
#[test]
fn mouse_payment_confirmation_uses_the_same_guarded_dispatch() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (mut app, mut rx, _) = app();
    book_form(&mut app, 1, "1", "cash");
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 25,
            row: 14,
            modifiers: KeyModifiers::NONE,
        },
        ratatui::layout::Rect::new(0, 0, 100, 40),
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        Command::Book {
            request: Request::Add(1.0),
            ..
        }
    ));
}
