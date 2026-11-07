use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::knob::{format_value, KNOB_SPECS};
use crate::tui::app::{App, FocusZone};

use super::{focused_border, list_state, panel, selection_style};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(4),
        ])
        .split(area);

    let filter = if app.animation.filter.is_empty() {
        String::new()
    } else {
        format!("  filter: {}", app.animation.filter)
    };
    let header = Line::from(vec![
        Span::raw("Edits are local until Enter; incoming knob/get updates non-dirty knobs."),
        Span::styled(filter, Style::default().fg(app.theme.muted)),
    ]);
    frame.render_widget(Paragraph::new(header), rows[0]);

    let visible = app.visible_knobs();
    let selected = visible
        .iter()
        .position(|index| *index == app.animation.focus);
    let items: Vec<ListItem> = visible
        .iter()
        .map(|index| {
            let spec = &KNOB_SPECS[*index];
            let value = app.animation.values[*index];
            let dirty = app.animation.dirty[*index];
            let ratio = value.map(|value| spec.ratio(value)).unwrap_or(0.0);
            let bar_width = 20usize;
            let filled = ((ratio * bar_width as f64).round() as usize).min(bar_width);
            let bar = format!(
                "{}{}",
                "█".repeat(filled),
                "░".repeat(bar_width.saturating_sub(filled))
            );
            let value_text = value.map(format_value).unwrap_or_else(|| "-".to_string());
            let name_style = Style::default().fg(if dirty {
                app.theme.warning
            } else {
                app.theme.foreground
            });
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", spec.name), name_style),
                Span::styled(
                    format!("{value_text:>8}"),
                    Style::default().fg(app.theme.accent),
                ),
                Span::raw(" "),
                Span::styled(
                    bar,
                    Style::default().fg(if dirty {
                        app.theme.warning
                    } else {
                        app.theme.accent
                    }),
                ),
                Span::styled(
                    if dirty { " *" } else { "" },
                    Style::default().fg(app.theme.warning),
                ),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(
            panel("Animation knobs").border_style(focused_border(
                &app.theme,
                app.zone == FocusZone::Content,
            )),
        )
        .highlight_style(selection_style(&app.theme))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, rows[1], &mut list_state(selected));

    let spec = &KNOB_SPECS[app.animation.focus];
    let committed = app.animation.committed[app.animation.focus];
    let dirty = app.animation.dirty[app.animation.focus];
    let open = match (spec.open_min, spec.open_max) {
        (true, true) => "both",
        (true, false) => "min",
        (false, true) => "max",
        (false, false) => "none",
    };
    let detail = vec![
        Line::from(format!(
            "{}: range {} .. {}   open: {}   step {:.2} / {:.2}",
            spec.name,
            format_value(spec.min),
            format_value(spec.max),
            open,
            spec.step_small(),
            spec.step_big()
        )),
        Line::from(vec![
            Span::styled(
                if dirty {
                    "modified — Enter sends, Esc reverts"
                } else {
                    "up to date"
                },
                Style::default().fg(if dirty {
                    app.theme.warning
                } else {
                    app.theme.muted
                }),
            ),
            Span::raw(format!(
                "  committed: {}",
                committed
                    .map(format_value)
                    .unwrap_or_else(|| "-".to_string())
            )),
        ]),
    ];
    frame.render_widget(Paragraph::new(detail).block(panel("Focused knob")), rows[2]);
}
