use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::tui::app::{App, FocusZone};

use super::{focused_border, list_state, panel, selection_style};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(4)])
        .split(area);

    let warning = vec![
        Line::from(Span::styled(
            "telinit off sends {\"args\":\"0\"} and stops services.",
            Style::default().fg(app.theme.error),
        )),
        Line::from("telinit on sends {\"args\":\"4\"}."),
        Line::from("Safe mode asks for confirmation before sending."),
    ];
    frame.render_widget(
        Paragraph::new(warning)
            .block(panel("Warning").border_style(focused_border(&app.theme, false))),
        rows[0],
    );

    let items = vec![
        ListItem::new(Line::from(vec![
            Span::styled("on  ", Style::default().fg(app.theme.success)),
            Span::raw("telinit 4"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("off ", Style::default().fg(app.theme.error)),
            Span::raw("telinit 0 (destructive)"),
        ])),
    ];
    let list = List::new(items)
        .block(
            panel("telinit")
                .border_style(focused_border(&app.theme, app.zone == FocusZone::Content)),
        )
        .highlight_style(selection_style(&app.theme))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, rows[1], &mut list_state(Some(app.system.focus)));
}
