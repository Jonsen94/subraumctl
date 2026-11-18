//! Serial HTTP account operations and cancellable payment forms, independent of MQTT.

use crossterm::event::{Event as InputEvent, KeyCode, KeyEvent};
use eyre::{Result, ensure, eyre};
use ratatui::widgets::ListState;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tui_input::{Input, backend::crossterm::EventHandler};

use super::app::{App, Popup, Section, StatusKind};
use super::safe_mode::{ConfirmChoice, PendingCommand};
use super::traffic::{TrafficDirection, TrafficEntry, now_hms};
use crate::strichliste::{Client, Options, Request, User, cents, parse_amount};

pub const ACTIONS: [&str; 4] = [
    "Refresh balance and rank",
    "Pay",
    "Add credit",
    "Send to another user",
];

#[derive(Clone, Debug)]
pub enum Command {
    Refresh,
    Search(String),
    Book {
        request: Request,
        comment: String,
    },
    Send {
        recipient: User,
        amount: f64,
        comment: String,
    },
}

impl Command {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Book { request, .. } => {
                ensure!(request.value()?.is_some(), "expected pay or add")
            }
            Self::Send { amount, .. } => {
                cents(*amount)?;
            }
            Self::Search(query) => ensure!(!query.trim().is_empty(), "enter a recipient name"),
            Self::Refresh => {}
        }
        Ok(())
    }

    pub fn preview_lines(&self) -> Vec<String> {
        match self {
            Self::Refresh => vec!["Read your balance and purchase rank".into()],
            Self::Search(query) => vec![format!("Search recipients: {}", query.escape_debug())],
            Self::Book { request, comment } => vec![
                match request {
                    Request::Pay(amount) => format!("Pay {amount} EUR from your balance"),
                    Request::Add(amount) => format!("Add {amount} EUR to your balance"),
                    _ => "Invalid booking".into(),
                },
                format!("Comment: {}", comment.escape_debug()),
            ],
            Self::Send {
                recipient,
                amount,
                comment,
            } => vec![
                format!(
                    "Send {amount} EUR to {} (id {})",
                    recipient.name.escape_debug(),
                    recipient.id
                ),
                format!("Comment: {}", comment.escape_debug()),
            ],
        }
    }
}

pub enum Response {
    Account {
        balance: Result<f64, String>,
        rank: Result<(u64, f64), String>,
    },
    Recipients(Vec<User>),
    Submitted,
    DryRun,
}

pub struct Event {
    pub command: Command,
    pub result: Result<Response, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pay,
    Add,
    Send,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Search,
    Loading,
    Recipients,
    Amount,
    Comment,
}

pub struct Form {
    pub kind: Kind,
    pub step: Step,
    pub input: Input,
    pub amount: Option<f64>,
    pub recipient: Option<User>,
    pub recipients: Vec<User>,
    pub selection: ListState,
}

impl Form {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            step: if kind == Kind::Send {
                Step::Search
            } else {
                Step::Amount
            },
            input: Input::default(),
            amount: None,
            recipient: None,
            recipients: Vec::new(),
            selection: ListState::default(),
        }
    }

    pub fn title(&self) -> &'static str {
        match self.kind {
            Kind::Pay => "Pay",
            Kind::Add => "Add credit",
            Kind::Send => "Send",
        }
    }
}

#[derive(Default)]
pub struct State {
    pub tx: Option<UnboundedSender<Command>>,
    pub focus: usize,
    pub balance: Option<f64>,
    pub rank: Option<(u64, f64)>,
    pub balance_stale: bool,
    pub rank_stale: bool,
    pub busy: bool,
    pub initial_read: bool,
    pub message: String,
    pub error: Option<String>,
    pub form: Option<Form>,
}

async fn execute(options: Options, command: &Command, dry_run: bool) -> Result<Response> {
    command.validate()?;
    if dry_run {
        return Ok(Response::DryRun);
    }
    let client = Client::new(options)?;
    match command {
        Command::Refresh => {
            let balance = client
                .execute(&Request::Balance, "")
                .await
                .and_then(|value| {
                    value["balance_eur"]
                        .as_f64()
                        .ok_or_else(|| eyre!("missing balance"))
                })
                .map_err(|error| error.to_string());
            let rank = client
                .execute(&Request::Rank, "")
                .await
                .and_then(|value| {
                    Ok((
                        value["rank"]
                            .as_u64()
                            .ok_or_else(|| eyre!("missing rank"))?,
                        value["weighted_count_of_purchases"]
                            .as_f64()
                            .ok_or_else(|| eyre!("missing purchase score"))?,
                    ))
                })
                .map_err(|error| error.to_string());
            Ok(Response::Account { balance, rank })
        }
        Command::Search(query) => {
            let current = client.current_user().await?;
            let query = query.to_lowercase();
            let mut users: Vec<_> = client
                .users()
                .await?
                .into_iter()
                .filter(|user| user.id != current.id && user.name.to_lowercase().contains(&query))
                .collect();
            users.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then(a.id.cmp(&b.id))
            });
            Ok(Response::Recipients(users))
        }
        Command::Book { request, comment } => {
            client.execute(request, comment).await?;
            Ok(Response::Submitted)
        }
        Command::Send {
            recipient,
            amount,
            comment,
        } => {
            client.transfer(recipient, *amount, comment).await?;
            Ok(Response::Submitted)
        }
    }
}

pub fn spawn_worker(
    options: Options,
    dry_run: bool,
    mut rx: UnboundedReceiver<Command>,
    tx: UnboundedSender<Event>,
) {
    tokio::spawn(async move {
        while let Some(command) = rx.recv().await {
            let result = execute(options.clone(), &command, dry_run)
                .await
                .map_err(|e| e.to_string());
            if tx.send(Event { command, result }).is_err() {
                break;
            }
        }
    });
}

impl App {
    pub(super) fn maybe_refresh_strichliste(&mut self) {
        if self.section == Section::Strichliste
            && self.options.connect
            && !self.strichliste.initial_read
            && !self.strichliste.busy
        {
            self.send_strichliste(Command::Refresh);
        }
    }

    pub(super) fn handle_strichliste_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.strichliste.focus =
                    (self.strichliste.focus + ACTIONS.len() - 1) % ACTIONS.len()
            }
            KeyCode::Down => self.strichliste.focus = (self.strichliste.focus + 1) % ACTIONS.len(),
            KeyCode::Home => self.strichliste.focus = 0,
            KeyCode::End => self.strichliste.focus = ACTIONS.len() - 1,
            KeyCode::Enter if !self.strichliste.busy => {
                if self.strichliste.focus == 0 {
                    self.send_strichliste(Command::Refresh);
                } else {
                    let kind = match self.strichliste.focus {
                        1 => Kind::Pay,
                        2 => Kind::Add,
                        _ => Kind::Send,
                    };
                    self.strichliste.form = Some(Form::new(kind));
                    self.strichliste.error = None;
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_strichliste_input(&mut self, key: KeyEvent) {
        let Some(mut form) = self.strichliste.form.take() else {
            return;
        };
        if key.code == KeyCode::Esc {
            self.strichliste.message = "Form cancelled".into();
            return;
        }
        match form.step {
            Step::Loading => {}
            Step::Recipients => {
                let count = form.recipients.len();
                let selected = form.selection.selected().unwrap_or(0);
                let index = match key.code {
                    KeyCode::Up => selected.saturating_sub(1),
                    KeyCode::Down => (selected + 1).min(count.saturating_sub(1)),
                    KeyCode::PageUp => selected.saturating_sub(10),
                    KeyCode::PageDown => (selected + 10).min(count.saturating_sub(1)),
                    KeyCode::Home => 0,
                    KeyCode::End => count.saturating_sub(1),
                    _ => selected,
                };
                form.selection.select((count > 0).then_some(index));
                match key.code {
                    KeyCode::Char('/') => {
                        form.step = Step::Search;
                        form.input = Input::default();
                    }
                    KeyCode::Enter if count > 0 => {
                        form.recipient = form.recipients.get(index).cloned();
                        form.step = Step::Amount;
                        form.input = Input::default();
                    }
                    _ => {}
                }
            }
            _ if key.code == KeyCode::Enter => match form.step {
                Step::Search => {
                    let query = form.input.value().trim().to_owned();
                    if query.is_empty() {
                        self.strichliste.error = Some("Enter a recipient name".into());
                    } else {
                        form.step = Step::Loading;
                        self.strichliste.form = Some(form);
                        self.send_strichliste(Command::Search(query));
                        return;
                    }
                }
                Step::Amount => match parse_amount(form.input.value()) {
                    Ok(amount) => {
                        form.amount = Some(amount);
                        form.input = Input::default();
                        form.step = Step::Comment;
                        self.strichliste.error = None;
                    }
                    Err(error) => self.strichliste.error = Some(error),
                },
                Step::Comment => {
                    let Some(amount) = form.amount else { return };
                    let comment = form.input.value().to_owned();
                    let command = match form.kind {
                        Kind::Pay => Command::Book {
                            request: Request::Pay(amount),
                            comment,
                        },
                        Kind::Add => Command::Book {
                            request: Request::Add(amount),
                            comment,
                        },
                        Kind::Send => {
                            let Some(recipient) = form.recipient else {
                                return;
                            };
                            Command::Send {
                                recipient,
                                amount,
                                comment,
                            }
                        }
                    };
                    if self.safe_mode {
                        self.popup = Some(Popup::ConfirmCommand {
                            pending: PendingCommand::Strichliste(command),
                            selected: ConfirmChoice::Cancel,
                        });
                    } else {
                        self.send_strichliste(command);
                    }
                    return;
                }
                _ => {}
            },
            _ => {
                form.input.handle_event(&InputEvent::Key(key));
            }
        }
        self.strichliste.form = Some(form);
    }

    pub(super) fn send_strichliste(&mut self, command: Command) {
        if self.strichliste.busy {
            self.set_status("Strichliste is busy", StatusKind::Info);
            return;
        }
        let error = command.validate().err().map(|error| error.to_string());
        let message = command.preview_lines().join(" | ");
        let error = error.or_else(|| {
            (!self
                .strichliste
                .tx
                .as_ref()
                .is_some_and(|tx| tx.send(command.clone()).is_ok()))
            .then(|| "Strichliste worker is not running".to_owned())
        });
        if let Some(error) = error {
            self.strichliste.error = Some(error.clone());
            if let Some(form) = &mut self.strichliste.form {
                if form.step == Step::Loading {
                    form.step = Step::Search;
                }
            }
            self.set_status(error, StatusKind::Error);
            return;
        }
        if matches!(command, Command::Refresh) {
            self.strichliste.initial_read = true;
        }
        if !self.options.dry_run && matches!(command, Command::Book { .. } | Command::Send { .. }) {
            self.strichliste.balance_stale = true;
            self.strichliste.rank_stale = true;
        }
        self.strichliste.busy = true;
        self.strichliste.error = None;
        self.strichliste.message = message.clone();
        self.traffic.push(traffic(
            if self.options.dry_run {
                TrafficDirection::Dry
            } else {
                TrafficDirection::Tx
            },
            message,
        ));
    }

    pub(super) fn handle_strichliste_event(&mut self, event: Event) {
        self.strichliste.busy = false;
        self.strichliste.error = None;
        let mut refresh = false;
        let message = match event.result {
            Ok(Response::Account { balance, rank }) => {
                let mut errors = Vec::new();
                self.strichliste.balance_stale = balance.is_err();
                self.strichliste.rank_stale = rank.is_err();
                match balance {
                    Ok(value) => self.strichliste.balance = Some(value),
                    Err(error) => errors.push(error),
                }
                match rank {
                    Ok(value) => self.strichliste.rank = Some(value),
                    Err(error) => errors.push(error),
                }
                if errors.is_empty() {
                    "Account refreshed".into()
                } else {
                    self.strichliste.error = Some(errors.join("; "));
                    "Refresh incomplete; previous values retained".into()
                }
            }
            Ok(Response::Recipients(users)) => {
                if let Some(form) = &mut self.strichliste.form {
                    if form.step == Step::Loading {
                        form.step = Step::Recipients;
                        form.selection.select((!users.is_empty()).then_some(0));
                        form.recipients = users;
                    }
                }
                "Select a recipient; / searches again".into()
            }
            Ok(Response::Submitted) => {
                refresh = true;
                "Transaction submitted".into()
            }
            Ok(Response::DryRun) => {
                if let Some(form) = &mut self.strichliste.form {
                    if form.step == Step::Loading {
                        form.step = Step::Search;
                    }
                }
                format!("dry-run: {}", event.command.preview_lines().join(" | "))
            }
            Err(error) => {
                if matches!(event.command, Command::Refresh) {
                    self.strichliste.balance_stale = true;
                    self.strichliste.rank_stale = true;
                }
                if let Some(form) = &mut self.strichliste.form {
                    if form.step == Step::Loading {
                        form.step = Step::Search;
                    }
                }
                self.strichliste.error = Some(error.clone());
                error
            }
        };
        self.strichliste.message = message.clone();
        self.traffic.push(traffic(
            if self.strichliste.error.is_some() {
                TrafficDirection::Error
            } else {
                TrafficDirection::Rx
            },
            message.clone(),
        ));
        self.set_status(
            message,
            if self.strichliste.error.is_some() {
                StatusKind::Error
            } else {
                StatusKind::Info
            },
        );
        if refresh {
            self.send_strichliste(Command::Refresh);
        }
    }

    #[cfg(feature = "tui-mouse")]
    pub(super) fn handle_strichliste_click(
        &mut self,
        mouse: crossterm::event::MouseEvent,
        area: ratatui::layout::Rect,
        double_click: bool,
    ) {
        let rows = super::views::strichliste::areas(area);
        let list = rows[2];
        if mouse.row <= list.y || mouse.row >= list.bottom().saturating_sub(1) {
            return;
        }
        let row = mouse.row.saturating_sub(list.y + 1) as usize;
        if let Some(form) = &mut self.strichliste.form {
            if form.step != Step::Recipients {
                return;
            }
            let index = form.selection.offset() + row;
            if index >= form.recipients.len() {
                return;
            }
            form.selection.select(Some(index));
            if double_click {
                self.handle_strichliste_input(KeyEvent::from(KeyCode::Enter));
            }
        } else if row < ACTIONS.len() {
            self.strichliste.focus = row;
            if double_click {
                self.handle_strichliste_key(KeyEvent::from(KeyCode::Enter));
            }
        }
    }
}

fn traffic(direction: TrafficDirection, payload: String) -> TrafficEntry {
    TrafficEntry {
        at: now_hms(),
        direction,
        transport: "http",
        target: "Strichliste".into(),
        payload,
        note: None,
        sections: Section::Overview.bit() | Section::Strichliste.bit(),
    }
}

#[cfg(test)]
mod tests;
