use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::action::Outlet;
use crate::tui::app::{App, FocusZone};

use super::{focused_border, panel, selection_style, status_style, status_text};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = Outlet::ALL
        .iter()
        .enumerate()
        .map(|(index, outlet)| {
            let on_style = if app.outlets.action == 0 {
                Style::default()
                    .fg(app.theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.muted)
            };
            let off_style = if app.outlets.action == 1 {
                Style::default()
                    .fg(app.theme.error)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.muted)
            };
            let last = app.outlets.last_sent[index];
            Row::new(vec![
                Cell::from((*outlet).label()),
                Cell::from(Span::styled("on", on_style)),
                Cell::from(Span::styled("off", off_style)),
                Cell::from(Span::styled(
                    status_text(last),
                    status_style(&app.theme, last),
                )),
            ])
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Outlet"),
        Cell::from("On"),
        Cell::from("Off"),
        Cell::from("Last"),
    ]);
    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        panel("Outlets").border_style(focused_border(
            &app.theme,
            app.zone == FocusZone::Content,
        )),
    )
    .row_highlight_style(selection_style(&app.theme));

    let mut state = TableState::default();
    state.select(Some(app.outlets.row));
    frame.render_stateful_widget(table, area, &mut state);
}
