use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{List, ListItem, Paragraph},
};

use super::super::app::{App, FocusZone};
use super::{focused_border, list_state, panel, selection_style};

pub(crate) fn areas(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area)
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = areas(area);
    let state = &app.soundboard;
    let focused = app.zone == FocusZone::Content;
    let width = rows[0].width.saturating_sub(2) as usize;
    let scroll = state.input.visual_scroll(width.max(1));
    frame.render_widget(
        Paragraph::new(state.input.value())
            .scroll((0, scroll.min(u16::MAX as usize) as u16))
            .block(
                panel(if state.editing {
                    "Search (editing)"
                } else {
                    "Search (/ or Enter to edit)"
                })
                .border_style(focused_border(&app.theme, focused && state.focus == 0)),
            ),
        rows[0],
    );
    if focused && state.editing && width > 0 && rows[0].height >= 3 {
        frame.set_cursor_position((
            rows[0].x
                + 1
                + state
                    .input
                    .visual_cursor()
                    .saturating_sub(scroll)
                    .min(width - 1) as u16,
            rows[0].y + 1,
        ));
    }
    frame.render_widget(
        Paragraph::new(format!(
            "Queried: {}",
            state.query.as_deref().unwrap_or("—")
        )),
        rows[1],
    );
    let message = if state.busy {
        format!("Busy: {}", state.message)
    } else if let Some(error) = &state.error {
        format!("Error: {error}")
    } else {
        state.message.clone()
    };
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::default().fg(if state.error.is_some() {
                app.theme.error
            } else {
                app.theme.muted
            }))
            .wrap(ratatui::widgets::Wrap { trim: false }),
        rows[2],
    );
    let items: Vec<ListItem> = state
        .matches
        .iter()
        .map(|sample| ListItem::new(sample.as_str()))
        .collect();
    let list = List::new(items)
        .block(
            panel(if state.truncated {
                "Results (truncated)"
            } else {
                "Results"
            })
            .border_style(focused_border(&app.theme, focused && state.focus > 0)),
        )
        .highlight_style(selection_style(&app.theme))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, rows[3], &mut list_state(state.focus.checked_sub(1)));
    frame.render_widget(
        Paragraph::new("/ search · ↑↓ PgUp/PgDn select · Enter play · Esc leave input"),
        rows[4],
    );
}
