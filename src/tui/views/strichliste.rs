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

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = areas(area);
    let state = &mut app.strichliste;
    let focused = app.zone == FocusZone::Content;
    let stale = |yes| if yes { " (previous)" } else { "" };
    let balance = state
        .balance
        .map(|value| format!("{value} EUR"))
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
        .block(panel("Strichliste")),
        rows[0],
    );
    let message = if state.busy {
        format!("Busy: {}", state.message)
    } else if let Some(error) = &state.error {
        format!("Error: {error}")
    } else if state.message.is_empty() {
        "Enter refreshes; account uses --strichliste-user or --strichliste-user-id".into()
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
        if form.step == Step::Recipients {
            let items: Vec<_> = form
                .recipients
                .iter()
                .map(|user| ListItem::new(format!("{} (id {})", user.name.escape_debug(), user.id)))
                .collect();
            if items.is_empty() {
                frame.render_widget(
                    Paragraph::new("No matches. Press / to search again.")
                        .block(panel("Recipients")),
                    rows[2],
                );
            } else {
                frame.render_stateful_widget(
                    List::new(items)
                        .block(panel("Recipients: Enter selects, / searches again"))
                        .highlight_symbol("> ")
                        .highlight_style(selection_style(&app.theme)),
                    rows[2],
                    &mut form.selection,
                );
            }
        } else {
            let label = match form.step {
                Step::Search => "Recipient name",
                Step::Loading => "Searching recipients...",
                Step::Amount => "Amount in EUR",
                Step::Comment => "Comment (optional; Enter submits)",
                Step::Recipients => unreachable!(),
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
            let width = parts[1].width.saturating_sub(2) as usize;
            let scroll = form.input.visual_scroll(width.max(1));
            frame.render_widget(
                Paragraph::new(form.input.value())
                    .scroll((0, scroll.min(u16::MAX as usize) as u16))
                    .block(panel(label).border_style(focused_border(&app.theme, focused))),
                parts[1],
            );
            if focused && form.step != Step::Loading && width > 0 && parts[1].height >= 3 {
                frame.set_cursor_position((
                    parts[1].x
                        + 1
                        + form
                            .input
                            .visual_cursor()
                            .saturating_sub(scroll)
                            .min(width - 1) as u16,
                    parts[1].y + 1,
                ));
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
        Paragraph::new(if state.form.is_some() {
            "Enter next/submit · Esc cancel · Ctrl+C quit"
        } else {
            "Up/Down select · Enter activate"
        }),
        rows[3],
    );
}
