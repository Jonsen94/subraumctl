use super::*;
use crate::{
    cli::TuiView,
    color::{Theme, ThemePreset},
    mqtt::MqttOptions,
    tui::TuiOptions,
};
use crossterm::event::KeyModifiers;
use tokio::sync::mpsc;

fn app() -> (App, UnboundedReceiver<Request>) {
    let (mqtt, _) = mpsc::unbounded_channel();
    let (http, _) = mpsc::unbounded_channel();
    let (pizza, rx) = mpsc::unbounded_channel();
    (
        App::new(
            TuiOptions {
                mqtt: MqttOptions::new("tcp://unused:1883"),
                view: TuiView::Pizzamat,
                connect: false,
                safe_mode: true,
                theme: Theme::preset(ThemePreset::Dark),
                theme_preset: ThemePreset::Dark,
                smoothie_url: String::new(),
                pizzamat: Options {
                    user: "alice".into(),
                    ..Options::default()
                },
                verbose: 0,
                dry_run: true,
            },
            mqtt,
            http,
            pizza,
        ),
        rx,
    )
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn choose(app: &mut App, operation: Operation) {
    app.pizzamat.focus = Operation::ALL
        .iter()
        .position(|op| *op == operation)
        .unwrap()
        + 1;
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
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn food_input_accepts_global_keys_and_writes_require_confirmation() {
    let (mut app, mut rx) = app();
    choose(&mut app, Operation::Add);
    for ch in "Quattro q [S] ? cdr".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    assert!(!app.should_quit);
    assert!(screen(&mut app).contains("Quattro q [S] ? cdr"));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.popup, Some(Popup::ConfirmPizza { .. })));
    assert!(rx.try_recv().is_err());
    press(&mut app, KeyCode::Char('y'));
    let request = rx.try_recv().unwrap();
    assert_eq!(request.args, "Quattro q [S] ? cdr");
    assert_eq!(request.user, "alice");
    assert!(app.pizzamat.busy);
    choose(&mut app, Operation::Clear);
    assert!(rx.try_recv().is_err());
    app.handle_pizzamat_event(Event {
        request,
        result: Ok("Your order".into()),
    });
    assert!(!app.pizzamat.busy);
    assert_eq!(app.pizzamat.response, "Your order");
}

#[test]
fn dry_run_reads_work_without_mqtt_and_cancelled_forms_do_not_send() {
    let (mut app, mut rx) = app();
    choose(&mut app, Operation::ServiceAdd);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Display name"));
    press(&mut app, KeyCode::Esc);
    assert!(app.text_input.is_none());
    assert!(rx.try_recv().is_err());
    choose(&mut app, Operation::Reset);
    press(&mut app, KeyCode::Enter); // default selection is Cancel
    assert!(rx.try_recv().is_err());
    choose(&mut app, Operation::List);
    assert!(app.popup.is_none());
    assert_eq!(rx.try_recv().unwrap().operation, Operation::List);
}

#[test]
fn live_requests_require_the_existing_mqtt_connection() {
    use crate::tui::app::ConnectionState;
    let (mut app, mut rx) = app();
    app.options.dry_run = false;
    choose(&mut app, Operation::List);
    assert!(rx.try_recv().is_err());
    assert!(app.status.as_ref().unwrap().text.contains("Not connected"));
    app.connection = ConnectionState::Connected {
        since: std::time::Instant::now(),
    };
    choose(&mut app, Operation::Clear);
    assert!(matches!(app.popup, Some(Popup::ConfirmPizza { .. })));
    app.connection = ConnectionState::Disconnected;
    press(&mut app, KeyCode::Char('y'));
    assert!(rx.try_recv().is_err());
    assert!(!app.pizzamat.busy);
}

#[test]
fn missing_identity_is_actionable_and_response_renders_at_minimum_size() {
    let (mut app, mut rx) = app();
    app.options.pizzamat.user.clear();
    choose(&mut app, Operation::Clear);
    assert!(app.status.as_ref().unwrap().text.contains("identity"));
    assert!(rx.try_recv().is_err());
    app.pizzamat.focus = 0;
    press(&mut app, KeyCode::Enter);
    for ch in "Quinn".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.options.pizzamat.user, "Quinn");
    app.pizzamat.response = "Quinn: Margherita".into();
    assert!(screen(&mut app).contains("Quinn: Margherita"));
}
