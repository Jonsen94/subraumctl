use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{List, ListItem, Paragraph, Wrap},
};

use super::super::app::{App, FocusZone};
use super::super::strichliste::{ACTIONS, Step};
use super::{focused_border, list_state, panel, selection_style};

pub(crate) fn areas(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area)
}

pub(crate) fn search_areas(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area)
}

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = areas(area);
    let state = &mut app.strichliste;
    let focused = app.zone == FocusZone::Content;
    let stale = |yes| if yes { " (previous)" } else { "" };
    let balance = state
        .balance
        .map(|value| {
            if state.full_precision {
                format!("{value} EUR")
            } else {
                let rounded = format!("{value:.2}");
                format!("{} EUR", if rounded == "-0.00" { "0.00" } else { &rounded })
            }
        })
        .unwrap_or_else(|| "not read".into());
    let rank = state
        .rank
        .map(|(rank, score)| format!("{rank} (weighted purchases: {score})"))
        .unwrap_or_else(|| "not read".into());
    frame.render_widget(
        Paragraph::new(format!(
            "Balance: {balance}{}\nRank: {rank}{}",
            stale(state.balance_stale),
            stale(state.rank_stale),
        ))
        .block(panel(if state.full_precision {
            "Strichliste · full precision (p: round)"
        } else {
            "Strichliste · rounded to cents (p: full)"
        })),
        rows[0],
    );
    let message = if state.busy {
        format!("Busy: {}", state.message)
    } else if let Some(error) = &state.error {
        format!("Error: {error}")
    } else if state.message.is_empty() {
        "r refreshes; account uses --strichliste-user or --strichliste-user-id".into()
    } else {
        state.message.clone()
    };
    frame.render_widget(
        Paragraph::new(message)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(if state.error.is_some() {
                app.theme.error
            } else {
                app.theme.muted
            })),
        rows[1],
    );

    if let Some(form) = &mut state.form {
        if matches!(form.step, Step::Search | Step::Loading | Step::Recipients) {
            let parts = search_areas(rows[2]);
            draw_input(
                frame,
                parts[0],
                &form.input,
                "Recipient name · live search",
                &app.theme,
                focused,
            );
            let items: Vec<_> = form
                .recipients
                .iter()
                .map(|user| ListItem::new(format!("{} (id {})", user.name.escape_debug(), user.id)))
                .collect();
            if form.step != Step::Recipients || items.is_empty() {
                frame.render_widget(
                    Paragraph::new(match form.step {
                        Step::Loading => "Loading recipients… You can type while waiting.",
                        Step::Search => "Recipients not loaded. Enter retries; Esc cancels.",
                        _ => "No matching recipients. Edit the search or / to clear.",
                    })
                    .block(panel("Recipients")),
                    parts[1],
                );
            } else {
                frame.render_stateful_widget(
                    List::new(items)
                        .block(panel("Recipients · Up/Down select · Enter continues"))
                        .highlight_symbol("> ")
                        .highlight_style(selection_style(&app.theme)),
                    parts[1],
                    &mut form.selection,
                );
            }
        } else {
            let label = match form.step {
                Step::Amount => "Amount in EUR",
                Step::Comment => "Comment (optional; Enter submits)",
                _ => unreachable!(),
            };
            let parts = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(rows[2]);
            let recipient = form
                .recipient
                .as_ref()
                .map(|user| format!(" to {} (id {})", user.name.escape_debug(), user.id))
                .unwrap_or_default();
            let amount = form
                .amount
                .map(|value| format!(" {value} EUR"))
                .unwrap_or_default();
            frame.render_widget(
                Paragraph::new(format!("{}{amount}{recipient}", form.title()))
                    .wrap(Wrap { trim: false }),
                parts[0],
            );
            draw_input(frame, parts[1], &form.input, label, &app.theme, focused);
        }
    } else if state.show_transactions {
        let title = if state.transactions_stale {
            "Recent transactions · EUR (previous)"
        } else {
            "Recent transactions · EUR"
        };
        match &state.transactions {
            Some(transactions) if !transactions.is_empty() => {
                let items: Vec<_> = transactions
                    .iter()
                    .map(|transaction| {
                        let amount = if state.full_precision {
                            format!("{:+}", transaction.value_eur)
                        } else {
                            format!("{:+.2}", transaction.value_eur)
                        };
                        ListItem::new(format!(
                            "{}  {amount}  {}",
                            transaction.created_at.escape_debug(),
                            transaction.kind(),
                        ))
                    })
                    .collect();
                frame.render_stateful_widget(
                    List::new(items)
                        .block(panel(title))
                        .highlight_symbol("> ")
                        .highlight_style(selection_style(&app.theme)),
                    rows[2],
                    &mut state.transaction_selection,
                );
            }
            transactions => {
                frame.render_widget(
                    Paragraph::new(if transactions.is_some() {
                        "No recent transactions."
                    } else {
                        "Transactions not read. Press r to refresh."
                    })
                    .wrap(Wrap { trim: false })
                    .block(panel(title)),
                    rows[2],
                );
            }
        }
    } else {
        frame.render_stateful_widget(
            List::new(ACTIONS)
                .block(panel("Actions").border_style(focused_border(&app.theme, focused)))
                .highlight_symbol("> ")
                .highlight_style(selection_style(&app.theme)),
            rows[2],
            &mut list_state(Some(state.focus)),
        );
    }
    frame.render_widget(
        Paragraph::new(if state.show_transactions {
            "Enter details · r refresh · p precision · t/Esc back"
        } else if state.form.is_some() {
            "Enter next/submit · Esc cancel · Ctrl+C quit"
        } else if app.options.connect {
            "Auto 10s · r refresh · p precision · t history"
        } else {
            "Auto off · r refresh · p precision · t history"
        }),
        rows[3],
    );
}

fn draw_input(
    frame: &mut Frame,
    area: Rect,
    input: &tui_input::Input,
    label: &str,
    theme: &crate::color::Theme,
    focused: bool,
) {
    let width = area.width.saturating_sub(2) as usize;
    let scroll = input.visual_scroll(width.max(1));
    frame.render_widget(
        Paragraph::new(input.value())
            .scroll((0, scroll.min(u16::MAX as usize) as u16))
            .block(panel(label).border_style(focused_border(theme, focused))),
        area,
    );
    if focused && width > 0 && area.height >= 3 {
        frame.set_cursor_position((
            area.x + 1 + input.visual_cursor().saturating_sub(scroll).min(width - 1) as u16,
            area.y + 1,
        ));
    }
}
