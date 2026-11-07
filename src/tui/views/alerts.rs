use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::{App, FocusZone};

use super::{focused_border, list_state, panel, selection_style};

const ACTIONS: [&str; 6] = [
    "Radiation: .make",
    "Radiation: .hack",
    "Sob: hackbuzzer",
    "Sob: dingdong",
    "Sob: chewbacca",
    "Sob: noot noot",
];

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(5)])
        .split(area);

    let alert_lines = vec![
        Line::from(vec![
            Span::styled("● ", Style::default().fg(app.alert_color())),
            Span::styled(app.alert_label(), Style::default().fg(app.alert_color())),
        ]),
        Line::from("topic: heimdallr/alert/condition"),
        Line::from(format!("dim: {}", app.alert.dim)),
    ];
    frame.render_widget(
        Paragraph::new(alert_lines).block(
            panel("Alert LED").border_style(focused_border(&app.theme, false)),
        ),
        rows[0],
    );

    let items: Vec<ListItem> = ACTIONS
        .iter()
        .map(|action| ListItem::new(*action))
        .collect();
    let list = List::new(items)
        .block(
            panel("Alert actions").border_style(focused_border(
                &app.theme,
                app.zone == FocusZone::Content,
            )),
        )
        .highlight_style(selection_style(&app.theme))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, rows[1], &mut list_state(Some(app.alerts.focus)));
}
