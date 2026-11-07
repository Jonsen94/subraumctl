use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, LineGauge, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::color::{color_to_hex, Theme, ThemeRole};
use crate::tui::app::{App, FocusZone};
use crate::tui::theme::ColorPicker;

use super::{focused_border, list_state, panel, selection_style};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    let items: Vec<ListItem> = ThemeRole::ALL
        .iter()
        .map(|role| {
            let color = app.theme.get(*role);
            ListItem::new(Line::from(vec![
                Span::styled("██ ", Style::default().fg(color)),
                Span::styled(
                    format!("{:<12}", role.title()),
                    Style::default().fg(app.theme.foreground),
                ),
                Span::raw(color_to_hex(color)),
                Span::raw("  "),
                Span::styled(role.cli_flag(), Style::default().fg(app.theme.muted)),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(
            panel("Theme roles").border_style(focused_border(
                &app.theme,
                app.zone == FocusZone::Content,
            )),
        )
        .highlight_style(selection_style(&app.theme))
        .highlight_symbol("> ");
    frame.render_stateful_widget(
        list,
        rows[0],
        &mut list_state(Some(app.theme_view.focus)),
    );

    let hint = vec![
        Line::from("Enter opens the keyboard color picker for the selected role."),
        Line::from("x resets the role; X resets the whole theme after confirmation."),
        Line::from("Changes apply live and can be reproduced with the CLI flags shown on the right."),
    ];
    frame.render_widget(Paragraph::new(hint), rows[1]);
}

#[allow(clippy::needless_range_loop)]
pub fn draw_picker(frame: &mut Frame, area: Rect, picker: &ColorPicker, theme: &Theme) {
    frame.render_widget(Clear, area);
    let block = panel(format!("Color picker — {}", picker.role.title()))
        .border_style(Style::default().fg(theme.accent));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    let swatch = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(chunks[0]);
    frame
        .buffer_mut()
        .set_style(swatch[0], Style::default().bg(picker.current_color()));
    let swatch_text = vec![
        Line::from(vec![
            Span::styled("hex: ", Style::default().fg(theme.muted)),
            Span::styled(picker.hex(), Style::default().fg(theme.accent)),
        ]),
        Line::from(format!("role: {}", picker.role.title())),
        Line::from(if picker.preview_original {
            "preview: original (Space toggles)"
        } else {
            "preview: edited (Space toggles)"
        }),
    ];
    frame.render_widget(Paragraph::new(swatch_text), swatch[1]);

    let mode_line = Line::from(vec![
        Span::styled(
            format!("mode: {}", picker.mode.title()),
            Style::default().fg(theme.accent),
        ),
        Span::raw("  (m changes)   "),
        Span::styled(
            format!(
                "component: {}",
                picker.component_labels()[picker.component.min(2)]
            ),
            Style::default().fg(theme.accent),
        ),
    ]);
    frame.render_widget(Paragraph::new(mode_line), chunks[1]);

    let values = picker.component_values();
    let ranges = picker.component_ranges();
    let labels = picker.component_labels();
    for index in 0..3 {
        let range = ranges[index];
        let ratio = if (range.1 - range.0).abs() < f64::EPSILON {
            0.0
        } else {
            ((values[index] - range.0) / (range.1 - range.0)).clamp(0.0, 1.0)
        };
        let selected = index == picker.component.min(2);
        let gauge = LineGauge::default()
            .ratio(ratio)
            .label(format!("{}: {:.0}", labels[index], values[index]))
            .line_set(if selected {
                symbols::line::THICK
            } else {
                symbols::line::NORMAL
            })
            .filled_style(Style::default().fg(if selected {
                theme.accent
            } else {
                theme.muted
            }))
            .unfilled_style(Style::default().fg(theme.border));
        frame.render_widget(gauge, chunks[2 + index]);
    }

    if let Some(palette) = &picker.palette {
        let palette_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(chunks[5]);
        if palette.editing {
            let line = Line::from(vec![
                Span::styled("filter: ", Style::default().fg(theme.muted)),
                Span::raw(palette.filter.value()),
            ]);
            frame.render_widget(Paragraph::new(line), palette_chunks[0]);
            let x = palette_chunks[0].x + 8 + palette.filter.visual_cursor() as u16;
            frame.set_cursor_position(ratatui::layout::Position::new(
                x.min(palette_chunks[0].right().saturating_sub(1)),
                palette_chunks[0].y,
            ));
        } else {
            frame.render_widget(
                Paragraph::new("/: filter   Enter: choose   Esc: close palette"),
                palette_chunks[0],
            );
        }

        let filtered = palette.filtered();
        let selected = if filtered.is_empty() {
            None
        } else {
            Some(palette.selected.min(filtered.len() - 1))
        };
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|(name, (red, green, blue))| {
                let color = Color::Rgb(*red, *green, *blue);
                ListItem::new(Line::from(vec![
                    Span::styled("██ ", Style::default().fg(color)),
                    Span::raw(*name),
                    Span::raw("  "),
                    Span::styled(color_to_hex(color), Style::default().fg(theme.muted)),
                ]))
            })
            .collect();
        let list = List::new(items)
            .block(panel("Named colors"))
            .highlight_style(selection_style(theme))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, palette_chunks[1], &mut list_state(selected));
    } else if let Some(input) = &picker.input {
        let line = Line::from(vec![
            Span::styled("value: ", Style::default().fg(theme.muted)),
            Span::raw(input.value()),
        ]);
        frame.render_widget(Paragraph::new(line), chunks[5]);
        let x = chunks[5].x + 7 + input.visual_cursor() as u16;
        frame.set_cursor_position(ratatui::layout::Position::new(
            x.min(chunks[5].right().saturating_sub(1)),
            chunks[5].y,
        ));
    } else {
        let help = vec![
            Line::from("p: named colors   r: reset role   Space: preview original"),
            Line::from("Tab: component   m: mode   Home/End: min/max   digits: exact value"),
            Line::from("Enter: apply   Esc: cancel"),
        ];
        frame.render_widget(Paragraph::new(help), chunks[5]);
    }

    let footer = Line::from(Span::styled(
        "m mode  Tab component  ←→ adjust  Shift big  p palette  Enter apply  Esc cancel",
        Style::default().fg(theme.muted),
    ));
    frame.render_widget(Paragraph::new(footer), chunks[6]);
}
