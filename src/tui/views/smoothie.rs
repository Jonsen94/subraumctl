use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Gauge, Paragraph, Row, Table, TableState};

use crate::action::SmoothieRelay;
use crate::smoothiesaur::TEMP_TOPICS;
use crate::tui::app::{App, FocusZone};

use super::{focused_border, panel, selection_style, status_style, status_text};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(columns[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Min(3),
        ])
        .split(columns[1]);

    draw_relays(frame, left[0], app);
    draw_laser(frame, left[1], app);
    draw_temperatures(frame, right[0], app);
    draw_flow_and_laser(frame, right[1], app);
    draw_safety(frame, right[2], app);
}

fn draw_relays(frame: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = SmoothieRelay::ALL
        .iter()
        .enumerate()
        .map(|(index, relay)| {
            let status = app.smoothie.status.relay(*relay);
            let choice = app.smoothie.relay_choice[index];
            let on_style = if choice {
                Style::default().fg(app.theme.success)
            } else {
                Style::default().fg(app.theme.muted)
            };
            let off_style = if choice {
                Style::default().fg(app.theme.muted)
            } else {
                Style::default().fg(app.theme.error)
            };
            Row::new(vec![
                Cell::from((*relay).label()),
                Cell::from(Span::styled(
                    status_text(status),
                    status_style(&app.theme, status),
                )),
                Cell::from(Line::from(vec![
                    Span::styled("on ", on_style),
                    Span::styled("off", off_style),
                ])),
            ])
        })
        .collect();
    let header = Row::new(vec![
        Cell::from("Relay"),
        Cell::from("Status"),
        Cell::from("Choice"),
    ]);
    let selected = (app.smoothie.focus < 7).then_some(app.smoothie.focus);
    let table = Table::new(
        rows,
        [
            Constraint::Min(16),
            Constraint::Length(7),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(panel("Relays").border_style(focused_border(
        &app.theme,
        app.zone == FocusZone::Content && app.smoothie.focus < 7,
    )))
    .row_highlight_style(selection_style(&app.theme));
    let mut state = TableState::default();
    state.select(selected);
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_laser(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(area);
    let gauge = Gauge::default()
        .block(panel("Laser power").border_style(focused_border(
            &app.theme,
            app.zone == FocusZone::Content && app.smoothie.focus == 7,
        )))
        .gauge_style(Style::default().fg(app.theme.accent))
        .ratio(app.smoothie.laser as f64 / 255.0)
        .label(format!("{}/255", app.smoothie.laser));
    frame.render_widget(gauge, chunks[0]);

    let apply_style = if app.zone == FocusZone::Content && app.smoothie.focus == 7 {
        selection_style(&app.theme)
    } else {
        Style::default().fg(app.theme.muted)
    };
    let zero_style = if app.zone == FocusZone::Content && app.smoothie.focus == 8 {
        selection_style(&app.theme)
    } else {
        Style::default().fg(app.theme.error)
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled("[ Apply ]", apply_style),
            Span::raw("  "),
            Span::styled("[ Zero ]", zero_style),
            Span::raw(format!(
                "  laser readout: {} / {} / {}",
                opt_percent(app.smoothie.status.laser.control_8),
                opt_percent(app.smoothie.status.laser.control_16),
                opt_volts(app.smoothie.status.laser.volts)
            )),
        ]),
        Line::from(format!(
            "last HTTP: {}",
            app.smoothie
                .last_http
                .as_deref()
                .unwrap_or("no HTTP command yet")
        )),
    ];
    if let Some(error) = &app.smoothie.http_error {
        lines.push(Line::from(Span::styled(
            format!("HTTP error: {error}"),
            Style::default().fg(app.theme.error),
        )));
    }
    frame.render_widget(Paragraph::new(lines), chunks[1]);
}

fn draw_temperatures(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = TEMP_TOPICS
        .iter()
        .enumerate()
        .map(|(index, (_, label))| {
            Line::from(vec![
                Span::styled(
                    format!("{:<26}", **label),
                    Style::default().fg(app.theme.muted),
                ),
                Span::raw(temp_text(app.smoothie.status.temps[index])),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("Temperatures").border_style(focused_border(&app.theme, false))),
        area,
    );
}

fn draw_flow_and_laser(frame: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(format!(
            "flow: raw={} rate={}",
            opt_i64(app.smoothie.status.flow_raw),
            opt_f64(app.smoothie.status.flow_rate)
        )),
        Line::from(format!(
            "laser: 8-bit={}  16-bit={}  voltage={}",
            opt_percent(app.smoothie.status.laser.control_8),
            opt_percent(app.smoothie.status.laser.control_16),
            opt_volts(app.smoothie.status.laser.volts)
        )),
        Line::from(format!(
            "last HTTP: {}",
            app.smoothie.last_http.as_deref().unwrap_or("(none)")
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("Flow / laser").border_style(focused_border(&app.theme, false))),
        area,
    );
}

fn draw_safety(frame: &mut Frame, area: Rect, app: &App) {
    let bits = app.smoothie.status.safety_bits();
    let known = app.smoothie.status.safety.is_some();
    let mut rows = Vec::new();
    for chunk in bits.chunks(2) {
        let mut cells = Vec::new();
        for (_, label, on) in chunk {
            let state = if known { status_text(Some(*on)) } else { "?" };
            cells.push(Cell::from(format!("{}: {state}", **label)));
        }
        while cells.len() < 2 {
            cells.push(Cell::from(""));
        }
        rows.push(Row::new(cells));
    }
    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .block(panel("Safety loop").border_style(focused_border(&app.theme, false)));
    frame.render_widget(table, area);
}

fn temp_text(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.2} °C"),
        None => "?".to_string(),
    }
}

fn opt_f64(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.2}"),
        None => "?".to_string(),
    }
}

fn opt_i64(value: Option<i64>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "?".to_string(),
    }
}

fn opt_percent(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.2}%"),
        None => "?".to_string(),
    }
}

fn opt_volts(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.4} V"),
        None => "?".to_string(),
    }
}
