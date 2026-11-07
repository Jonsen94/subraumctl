use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::action::{ControlCommand, DmxAnimation, DmxGroup};
use crate::tui::app::{App, FocusZone};

use super::{focused_border, list_state, panel, selection_style};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(rows[0]);

    let visible = app.visible_lighting_groups();
    let selected = visible
        .iter()
        .position(|index| *index == app.lighting.group);
    let group_title = if app.lighting.filter.is_empty() {
        "Groups".to_string()
    } else {
        format!("Groups — filter: {}", app.lighting.filter)
    };

    if visible.is_empty() {
        frame.render_widget(
            Paragraph::new("no matching groups").block(panel(group_title)),
            columns[0],
        );
    } else {
        let items: Vec<ListItem> = visible
            .iter()
            .map(|index| ListItem::new(DmxGroup::ALL[*index].as_str()))
            .collect();
        let list = List::new(items)
            .block(
                panel(group_title).border_style(focused_border(
                    &app.theme,
                    app.zone == FocusZone::Content,
                )),
            )
            .highlight_style(selection_style(&app.theme))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, columns[0], &mut list_state(selected));
    }

    let animation_items: Vec<ListItem> = DmxAnimation::ALL
        .iter()
        .map(|animation| ListItem::new((*animation).label()))
        .collect();
    let animation_list = List::new(animation_items)
        .block(
            panel("Animations").border_style(focused_border(
                &app.theme,
                app.zone == FocusZone::Content,
            )),
        )
        .highlight_style(selection_style(&app.theme))
        .highlight_symbol("> ");
    frame.render_stateful_widget(
        animation_list,
        columns[1],
        &mut list_state(Some(app.lighting.animation)),
    );

    let command = ControlCommand::Dmx {
        group: DmxGroup::ALL[app.lighting.group],
        animation: DmxAnimation::ALL[app.lighting.animation],
    };
    let preview = command
        .expand(&app.options.smoothie_url)
        .ok()
        .and_then(|actions| actions.first().map(|action| action.describe()))
        .unwrap_or_default();
    let lines = vec![
        Line::from(vec![
            Span::styled("Send: ", Style::default().fg(app.theme.muted)),
            Span::styled(preview, Style::default().fg(app.theme.accent)),
        ]),
        Line::from(vec![
            Span::raw("Enter sends; safe mode asks for confirmation. "),
            Span::styled("/ filters groups", Style::default().fg(app.theme.muted)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), rows[1]);
}
