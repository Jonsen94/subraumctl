use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Gauge, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use crate::tui::app::{App, FocusZone};
use crate::white_light;

use super::{focused_border, panel, selection_style, status_style, status_text};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(5),
        ])
        .split(area);
    let gauges = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let warm = Gauge::default()
        .block(
            panel("warm").border_style(focused_border(
                &app.theme,
                app.zone == FocusZone::Content && app.white_light.focus == 0,
            )),
        )
        .gauge_style(Style::default().fg(app.theme.accent))
        .ratio(app.white_light.warm as f64 / 255.0)
        .label(format!("{}", app.white_light.warm));
    frame.render_widget(warm, gauges[0]);

    let cold = Gauge::default()
        .block(
            panel("cold").border_style(focused_border(
                &app.theme,
                app.zone == FocusZone::Content && app.white_light.focus == 1,
            )),
        )
        .gauge_style(Style::default().fg(app.theme.accent))
        .ratio(app.white_light.cold as f64 / 255.0)
        .label(format!("{}", app.white_light.cold));
    frame.render_widget(cold, gauges[1]);

    let selected_count = app
        .white_light
        .selected
        .iter()
        .filter(|selected| **selected)
        .count();
    let apply_style = if app.zone == FocusZone::Content && app.white_light.focus == 2 {
        selection_style(&app.theme)
    } else {
        Style::default().fg(app.theme.muted)
    };
    let apply_line = Line::from(vec![
        Span::styled("[ Apply ]", apply_style),
        Span::raw(format!(
            "  {selected_count} cell(s) selected  w={} c={}",
            app.white_light.warm, app.white_light.cold
        )),
    ]);
    frame.render_widget(Paragraph::new(apply_line), rows[1]);

    let visible = app.visible_white_light_cells();
    let selected = app
        .white_light
        .focus
        .checked_sub(3)
        .and_then(|index| visible.iter().position(|cell_index| *cell_index == index));
    let table_rows: Vec<Row> = visible
        .iter()
        .map(|index| {
            let cell = &white_light::CELLS[*index];
            let is_selected = app.white_light.selected[*index];
            let status = app.white_light.status[*index];
            Row::new(vec![
                Cell::from(Span::styled(
                    if is_selected { "[x]" } else { "[ ]" },
                    Style::default().fg(if is_selected {
                        app.theme.accent
                    } else {
                        app.theme.muted
                    }),
                )),
                Cell::from(Span::styled(
                    status_text(status),
                    status_style(&app.theme, status),
                )),
                Cell::from(cell.group.label()),
                Cell::from(cell.id),
            ])
        })
        .collect();
    let header = Row::new(vec![
        Cell::from("Sel"),
        Cell::from("Status"),
        Cell::from("Group"),
        Cell::from("Cell"),
    ]);
    let table = Table::new(
        table_rows,
        [
            Constraint::Length(4),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        panel(if app.white_light.filter.is_empty() {
            format!("Cells — {selected_count} selected")
        } else {
            format!(
                "Cells — {selected_count} selected — filter: {}",
                app.white_light.filter
            )
        })
        .border_style(focused_border(
            &app.theme,
            app.zone == FocusZone::Content,
        )),
    )
    .row_highlight_style(selection_style(&app.theme));

    let mut state = TableState::default();
    state.select(selected);
    frame.render_stateful_widget(table, rows[2], &mut state);
}
