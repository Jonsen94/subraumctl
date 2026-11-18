use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Paragraph, Widget, Wrap,
        canvas::{Canvas, Line as CanvasLine, Points},
    },
};

use super::{
    Domain,
    history::{History, Series},
};
use crate::{
    cli::GraphLayout,
    tui::{
        app::{App, FocusZone},
        views::{focused_border, panel},
    },
};

#[derive(Debug, PartialEq, Eq)]
struct PlotKey {
    area: Rect,
    layout: GraphLayout,
    humidity: bool,
    scroll: usize,
    foreground: Color,
    background: Color,
}

#[derive(Debug)]
pub(super) struct PlotCache {
    key: PlotKey,
    buffer: Buffer,
    #[cfg(test)]
    pub(super) renders: usize,
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, domain: Domain) {
    let state = app.graphs.state(domain);
    let areas = Layout::vertical([Constraint::Length(6), Constraint::Min(0)]).split(area);
    let layout = if state.layout == GraphLayout::Overlay {
        "overlay"
    } else {
        "separate"
    };
    let age = state.updated.map_or_else(
        || "not yet read".into(),
        |at| format!("refreshed {}s ago", at.elapsed().as_secs()),
    );
    let status = if let Some(error) = &state.error {
        format!("STALE / refresh failed: {error}")
    } else if state.busy {
        "Reading history…".into()
    } else if app.options.dry_run {
        "Dry-run: no HTTP requests".into()
    } else {
        age
    };
    let metric = match domain {
        Domain::Power => "Power (W)",
        Domain::Climate if state.humidity => "Humidity (%)",
        Domain::Climate => "Temperature (°C)",
    };
    let end_age = state.history.as_ref().map_or_else(String::new, |history| {
        let age = crate::influx::now_ms()
            .unwrap_or(history.end_ms)
            .saturating_sub(history.end_ms)
            / 1000;
        format!(" | end: {age}s ago")
    });
    frame.render_widget(
        Paragraph::new(format!(
            "{metric} | {layout} | requested: {}m\n{status}{end_age}\ng: table/overlay/separate  +/-: range\nEnter: refresh  ↑/↓: charts{}",
            state.minutes, if domain == Domain::Climate { "  t: °C/%" } else { "" }
        ))
        .style(Style::default().fg(if state.error.is_some() { app.theme.error } else { app.theme.foreground }))
        .block(panel(format!("{} graphs", domain.section().title())).border_style(focused_border(&app.theme, app.zone == FocusZone::Content))),
        areas[0],
    );
    let Some(history) = &state.history else {
        frame.render_widget(
            Paragraph::new("No history loaded. Press Enter to read.").block(panel("History")),
            areas[1],
        );
        return;
    };
    let key = PlotKey {
        area: areas[1],
        layout: state.layout,
        humidity: state.humidity,
        scroll: state.scroll,
        foreground: app.theme.foreground,
        background: app.theme.background,
    };
    let mut cache = history.plot.borrow_mut();
    if cache.as_ref().is_none_or(|cached| cached.key != key) {
        let mut buffer = Buffer::empty(key.area);
        buffer.set_style(
            key.area,
            Style::default().fg(key.foreground).bg(key.background),
        );
        draw_history(&mut buffer, key.area, app, domain, history, metric);
        #[cfg(test)]
        let renders = cache.as_ref().map_or(1, |cached| cached.renders + 1);
        *cache = Some(PlotCache {
            key,
            buffer,
            #[cfg(test)]
            renders,
        });
    }
    // Terminal frames are cleared before drawing. Copy the cached cells rather
    // than rebuilding datasets and rasterizing every time any TUI event arrives.
    let cached = &cache.as_ref().expect("plot rendered").buffer;
    let target = frame.buffer_mut();
    for y in cached.area.top()..cached.area.bottom() {
        for x in cached.area.left()..cached.area.right() {
            target[(x, y)] = cached[(x, y)].clone();
        }
    }
}

fn draw_history(
    buffer: &mut Buffer,
    area: Rect,
    app: &App,
    domain: Domain,
    history: &History,
    metric: &str,
) {
    let state = app.graphs.state(domain);
    let lines: Vec<_> = history
        .series
        .iter()
        .filter(|s| s.humidity == state.humidity)
        .collect();
    let title = format!("{metric} — last {}m (bucket means)", history.minutes);
    if lines.is_empty() {
        Paragraph::new("No history returned for this metric.")
            .block(panel(title))
            .render(area, buffer);
    } else if state.layout == GraphLayout::Overlay {
        chart(buffer, area, app, history, &lines, &title);
    } else {
        let start = state.scroll.min(lines.len().saturating_sub(1));
        let count = (area.height as usize / 9)
            .clamp(1, 4)
            .min(lines.len() - start);
        let charts = Layout::vertical(vec![Constraint::Ratio(1, count as u32); count]).split(area);
        for (offset, line) in lines.iter().skip(start).take(count).enumerate() {
            chart(
                buffer,
                charts[offset],
                app,
                history,
                &[*line],
                &format!(
                    "{} — {title} [{}/{}]",
                    line.name,
                    start + offset + 1,
                    lines.len()
                ),
            );
        }
    }
}

fn color(history: &History, name: &str) -> Color {
    // Same room color in both metrics and layouts, including missing metrics.
    let names: std::collections::BTreeSet<_> =
        history.series.iter().map(|s| s.name.as_str()).collect();
    let index = names.iter().position(|n| *n == name).unwrap_or(0);
    let palette = [
        Color::Cyan,
        Color::Yellow,
        Color::Magenta,
        Color::Green,
        Color::Blue,
        Color::Red,
    ];
    palette[index % palette.len()]
}

fn chart(
    buffer: &mut Buffer,
    area: Rect,
    app: &App,
    history: &History,
    lines: &[&Series],
    title: &str,
) {
    if area.width < 20 || area.height < 6 {
        Paragraph::new("Enlarge terminal for graph")
            .block(panel(title))
            .render(area, buffer);
        return;
    }
    // A horizontal legend stays visible at the TUI's 80x24 minimum, where
    // A floating legend would otherwise disappear or obscure the plot.
    let legend = Paragraph::new(Line::from(
        lines
            .iter()
            .map(|line| {
                Span::styled(
                    format!(
                        " {}{} ",
                        line.name,
                        if line.segments.is_empty() { " (?)" } else { "" }
                    ),
                    Style::default().fg(color(history, &line.name)),
                )
            })
            .collect::<Vec<_>>(),
    ))
    .wrap(Wrap { trim: false });
    let legend_height = (legend.line_count(area.width) as u16).min(area.height.saturating_sub(6));
    let areas =
        Layout::vertical([Constraint::Min(6), Constraint::Length(legend_height)]).split(area);
    legend.render(areas[1], buffer);
    let area = areas[0];
    let (low, high) = lines
        .iter()
        .flat_map(|s| &s.segments)
        .flatten()
        .map(|(_, y)| *y)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), y| {
            (low.min(y), high.max(y))
        });
    if !low.is_finite() {
        Paragraph::new("No samples in this time window.")
            .block(panel(title))
            .render(area, buffer);
        return;
    }
    let padding = ((high - low) * 0.05).max(0.1);
    let bounds = [low - padding, high + padding];
    let block = panel(title);
    let inner = block.inner(area);
    block.render(area, buffer);
    let labels = [format!("{:.1}", bounds[0]), format!("{:.1}", bounds[1])];
    let label_width = labels.iter().map(String::len).max().unwrap_or(0) as u16;
    let margin = (label_width + 1).min(inner.width / 3);
    let plot = Rect::new(
        inner.x + margin + 1,
        inner.y,
        inner.width.saturating_sub(margin + 1),
        inner.height.saturating_sub(2),
    );
    if plot.width == 0 || plot.height == 0 {
        return;
    }
    for y in plot.top()..plot.bottom() {
        buffer[(plot.x - 1, y)].set_symbol("│");
    }
    for x in plot.left()..plot.right() {
        buffer[(x, plot.bottom())].set_symbol("─");
    }
    buffer[(plot.x - 1, plot.bottom())].set_symbol("└");
    for (label, y) in [(&labels[0], plot.bottom() - 1), (&labels[1], plot.top())] {
        Paragraph::new(label.as_str())
            .right_aligned()
            .render(Rect::new(inner.x, y, margin, 1), buffer);
    }
    let label_area = Rect::new(plot.x, plot.bottom() + 1, plot.width, 1);
    Paragraph::new(format!("-{}m", history.minutes)).render(label_area, buffer);
    if plot.width >= 24 {
        Paragraph::new(format!("-{:.1}m", history.minutes as f64 / 2.0))
            .centered()
            .render(label_area, buffer);
    }
    Paragraph::new("0m")
        .right_aligned()
        .render(label_area, buffer);
    let colors: Vec<_> = lines
        .iter()
        .map(|line| color(history, &line.name))
        .collect();
    plot_series(
        buffer,
        plot,
        history.minutes,
        bounds,
        lines,
        &colors,
        app.theme.background,
    );
}

fn plot_series(
    buffer: &mut Buffer,
    area: Rect,
    minutes: u64,
    bounds: [f64; 2],
    lines: &[&Series],
    colors: &[Color],
    background: Color,
) {
    Canvas::default()
        .marker(Marker::Braille)
        .background_color(background)
        .x_bounds([-(minutes as f64), 0.0])
        .y_bounds(bounds)
        .paint(|ctx| {
            for (index, (line, &color)) in lines.iter().zip(colors).enumerate() {
                // One layer per target, not per segment. In Chart, each Dataset
                // switches markers and allocates another full-plot canvas layer.
                if index > 0 {
                    ctx.layer();
                }
                for segment in &line.segments {
                    ctx.draw(&Points {
                        coords: segment,
                        color,
                    });
                    for pair in segment.windows(2) {
                        ctx.draw(&CanvasLine {
                            x1: pair[0].0,
                            y1: pair[0].1,
                            x2: pair[1].0,
                            y2: pair[1].1,
                            color,
                        });
                    }
                }
            }
        })
        .render(area, buffer);
}

#[cfg(test)]
mod tests;
