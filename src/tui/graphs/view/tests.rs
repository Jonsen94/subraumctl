use super::*;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType};
use std::{hint::black_box, time::Instant};

fn series(segments: Vec<Vec<(f64, f64)>>) -> Series {
    Series {
        name: "test".into(),
        humidity: false,
        segments,
    }
}

#[test]
fn plot_keeps_gaps_singletons_and_negative_values() {
    let area = Rect::new(0, 0, 61, 11);
    let mut buffer = Buffer::empty(area);
    let line = series(vec![
        vec![(-60.0, -1.0), (-45.0, -1.0)],
        vec![(-15.0, 1.0), (0.0, 1.0)],
    ]);
    let singleton = series(vec![vec![(-30.0, 0.0)]]);
    plot_series(
        &mut buffer,
        area,
        60,
        [-2.0, 2.0],
        &[&line, &singleton],
        &[Color::Cyan, Color::Yellow],
        Color::Black,
    );
    let ink_at = |x| (0..area.height).any(|y| buffer[(x, y)].symbol() != " ");
    assert!(ink_at(5), "first segment should be connected");
    assert!(ink_at(55), "second segment should be connected");
    assert!(!ink_at(22), "must not connect across the gap");
    assert!(!ink_at(38), "must not connect across the gap");
    assert!(ink_at(30), "isolated samples must remain visible");
    assert_eq!(buffer[(30, 5)].fg, Color::Yellow);
    assert!(buffer.content.iter().any(|cell| cell.fg == Color::Cyan));
}

#[test]
fn same_target_segments_share_a_braille_cell_without_erasing_points() {
    let area = Rect::new(0, 0, 10, 5);
    let points = [(-60.0, 0.0), (-59.0, 1.0)];
    let fragmented = series(points.iter().map(|point| vec![*point]).collect());
    let mut actual = Buffer::empty(area);
    plot_series(
        &mut actual,
        area,
        60,
        [0.0, 20.0],
        &[&fragmented],
        &[Color::Cyan],
        Color::Black,
    );
    let mut expected = Buffer::empty(area);
    Canvas::default()
        .marker(Marker::Braille)
        .background_color(Color::Black)
        .x_bounds([-60.0, 0.0])
        .y_bounds([0.0, 20.0])
        .paint(|ctx| {
            ctx.draw(&Points {
                coords: &points,
                color: Color::Cyan,
            })
        })
        .render(area, &mut expected);
    assert_eq!(actual, expected);
}

// Run manually in release mode; no timing assertions on shared CI machines.
// Same bounds, canvas dimensions, colors and samples for both renderers.
#[test]
#[ignore = "rendering benchmark: cargo test --release benchmark_gap_rendering -- --ignored --nocapture"]
fn benchmark_gap_rendering() {
    for (width, height) in [(80, 24), (120, 46), (240, 80)] {
        for sparse in [false, true] {
            let series: Vec<_> = (0..4)
                .map(|index| {
                    let points: Vec<_> = (0..600)
                        .filter(|i| !sparse || i % 2 == 0)
                        .map(|i| {
                            (
                                -60.0 + i as f64 / 10.0,
                                index as f64 + (i as f64 / 20.0).sin(),
                            )
                        })
                        .collect();
                    series(if sparse {
                        points.into_iter().map(|point| vec![point]).collect()
                    } else {
                        vec![points]
                    })
                })
                .collect();
            let lines: Vec<_> = series.iter().collect();
            let colors = [Color::Cyan, Color::Yellow, Color::Magenta, Color::Green];
            let area = Rect::new(0, 0, width, height);
            let mut buffer = Buffer::empty(area);
            let iterations = 10;
            let start = Instant::now();
            for _ in 0..iterations {
                buffer.reset();
                let datasets = lines
                    .iter()
                    .zip(colors)
                    .flat_map(|(line, color)| {
                        line.segments.iter().map(move |segment| {
                            Dataset::default()
                                .marker(Marker::Braille)
                                .graph_type(GraphType::Line)
                                .style(Style::default().fg(color))
                                .data(segment)
                        })
                    })
                    .collect();
                Chart::new(datasets)
                    .style(Style::default().bg(Color::Black))
                    .x_axis(Axis::default().bounds([-60.0, 0.0]))
                    .y_axis(Axis::default().bounds([-2.0, 5.0]))
                    .legend_position(None)
                    .render(area, &mut buffer);
                black_box(&buffer);
            }
            let old = start.elapsed() / iterations;
            let start = Instant::now();
            for _ in 0..iterations {
                buffer.reset();
                plot_series(
                    &mut buffer,
                    area,
                    60,
                    [-2.0, 5.0],
                    &lines,
                    &colors,
                    Color::Black,
                );
                black_box(&buffer);
            }
            let new = start.elapsed() / iterations;
            eprintln!(
                "{width}x{height}, sparse={sparse}: old={old:?}, new={new:?}, speedup={:.1}x",
                old.as_secs_f64() / new.as_secs_f64()
            );
        }
    }
}
