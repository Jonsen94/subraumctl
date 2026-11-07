//! Keyboard-driven color picker for theme roles.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
use tui_input::Input;

use crate::color::{ThemeRole, color_to_hex, hsl_to_rgb, hsv_to_rgb, rgb_to_hsl, rgb_to_hsv};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Rgb,
    Hsv,
    Hsl,
}

impl ColorMode {
    pub fn next(self) -> Self {
        match self {
            Self::Rgb => Self::Hsv,
            Self::Hsv => Self::Hsl,
            Self::Hsl => Self::Rgb,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Hsv => "HSV",
            Self::Hsl => "HSL",
        }
    }

    pub fn labels(self) -> [&'static str; 3] {
        match self {
            Self::Rgb => ["R", "G", "B"],
            Self::Hsv => ["H", "S", "V"],
            Self::Hsl => ["H", "S", "L"],
        }
    }
}

pub const NAMED_COLORS: [(&str, &str); 24] = [
    ("black", "#000000"),
    ("white", "#ffffff"),
    ("red", "#ff0000"),
    ("green", "#008000"),
    ("blue", "#0000ff"),
    ("yellow", "#ffff00"),
    ("cyan", "#00ffff"),
    ("magenta", "#ff00ff"),
    ("orange", "#ffa500"),
    ("purple", "#800080"),
    ("teal", "#008080"),
    ("navy", "#000080"),
    ("maroon", "#800000"),
    ("olive", "#808000"),
    ("lime", "#00ff00"),
    ("aqua", "#00ffff"),
    ("fuchsia", "#ff00ff"),
    ("silver", "#c0c0c0"),
    ("gray", "#808080"),
    ("gold", "#ffd700"),
    ("pink", "#ffc0cb"),
    ("brown", "#a52a2a"),
    ("rebeccapurple", "#663399"),
    ("darkgreen", "#006400"),
];

pub struct PaletteState {
    pub selected: usize,
    pub filter: Input,
    pub editing: bool,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            selected: 0,
            filter: Input::default(),
            editing: false,
        }
    }
}

impl PaletteState {
    pub fn filtered(&self) -> Vec<(&'static str, (u8, u8, u8))> {
        let needle = self.filter.value().trim().to_ascii_lowercase();
        NAMED_COLORS
            .iter()
            .filter(|(name, _)| needle.is_empty() || name.contains(needle.as_str()))
            .map(|(name, hex)| (*name, hex_to_rgb(hex)))
            .collect()
    }

    pub fn selected_color(&self) -> Option<(u8, u8, u8)> {
        let filtered = self.filtered();
        if filtered.is_empty() {
            return None;
        }
        let index = self.selected.min(filtered.len() - 1);
        filtered.get(index).map(|(_, rgb)| *rgb)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PaletteOutcome {
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.filter = Input::default();
                    self.editing = false;
                    self.selected = 0;
                }
                KeyCode::Enter => {
                    self.editing = false;
                }
                _ => {
                    self.filter.handle_event(&Event::Key(key));
                    self.selected = 0;
                }
            }
            return PaletteOutcome::Stay;
        }

        match key.code {
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                PaletteOutcome::Stay
            }
            KeyCode::Down => {
                let len = self.filtered().len();
                if len > 0 {
                    self.selected = (self.selected + 1).min(len - 1);
                }
                PaletteOutcome::Stay
            }
            KeyCode::Char('/') => {
                self.editing = true;
                PaletteOutcome::Stay
            }
            KeyCode::Enter => match self.selected_color() {
                Some(rgb) => PaletteOutcome::Selected(rgb),
                None => PaletteOutcome::Stay,
            },
            KeyCode::Esc => PaletteOutcome::Close,
            _ => PaletteOutcome::Stay,
        }
    }
}

pub enum PaletteOutcome {
    Stay,
    Selected((u8, u8, u8)),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PickerOutcome {
    None,
    Changed,
    Reset,
    Apply,
    Cancel,
}

pub struct ColorPicker {
    pub role: ThemeRole,
    pub original_color: Color,
    pub rgb: (u8, u8, u8),
    pub mode: ColorMode,
    pub component: usize,
    pub input: Option<Input>,
    pub palette: Option<PaletteState>,
    pub preview_original: bool,
    pub default_color: Color,
}

impl ColorPicker {
    pub fn new(role: ThemeRole, current: Color, default_color: Color) -> Self {
        let rgb = color_to_rgb(current);
        Self {
            role,
            original_color: current,
            rgb,
            mode: ColorMode::Rgb,
            component: 0,
            input: None,
            palette: None,
            preview_original: false,
            default_color,
        }
    }

    pub fn current_color(&self) -> Color {
        if self.preview_original {
            self.original_color
        } else {
            Color::Rgb(self.rgb.0, self.rgb.1, self.rgb.2)
        }
    }

    pub fn hex(&self) -> String {
        color_to_hex(self.current_color())
    }

    pub fn component_labels(&self) -> [&'static str; 3] {
        self.mode.labels()
    }

    pub fn component_values(&self) -> [f64; 3] {
        match self.mode {
            ColorMode::Rgb => [self.rgb.0 as f64, self.rgb.1 as f64, self.rgb.2 as f64],
            ColorMode::Hsv => {
                let (hue, saturation, value) = rgb_to_hsv(self.rgb.0, self.rgb.1, self.rgb.2);
                [hue, saturation * 100.0, value * 100.0]
            }
            ColorMode::Hsl => {
                let (hue, saturation, lightness) = rgb_to_hsl(self.rgb.0, self.rgb.1, self.rgb.2);
                [hue, saturation * 100.0, lightness * 100.0]
            }
        }
    }

    pub fn component_ranges(&self) -> [(f64, f64); 3] {
        match self.mode {
            ColorMode::Rgb => [(0.0, 255.0), (0.0, 255.0), (0.0, 255.0)],
            ColorMode::Hsv | ColorMode::Hsl => [(0.0, 360.0), (0.0, 100.0), (0.0, 100.0)],
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome {
        if let Some(mut palette) = self.palette.take() {
            match palette.handle_key(key) {
                PaletteOutcome::Stay => {
                    self.palette = Some(palette);
                }
                PaletteOutcome::Selected(rgb) => {
                    self.rgb = rgb;
                    self.preview_original = false;
                    return PickerOutcome::Changed;
                }
                PaletteOutcome::Close => {}
            }
            return PickerOutcome::None;
        }

        if let Some(mut input) = self.input.take() {
            match key.code {
                KeyCode::Esc => return PickerOutcome::None,
                KeyCode::Enter => {
                    if let Ok(value) = input.value().trim().parse::<f64>() {
                        self.set_component_value(self.component, value);
                        return PickerOutcome::Changed;
                    }
                    return PickerOutcome::None;
                }
                _ => {
                    input.handle_event(&Event::Key(key));
                    self.input = Some(input);
                    return PickerOutcome::None;
                }
            }
        }

        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Char('m') => {
                self.mode = self.mode.next();
                self.component = 0;
                PickerOutcome::None
            }
            KeyCode::Tab => {
                self.component = (self.component + 1) % 3;
                PickerOutcome::None
            }
            KeyCode::BackTab => {
                self.component = (self.component + 2) % 3;
                PickerOutcome::None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.adjust(-self.step(shift));
                PickerOutcome::Changed
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.adjust(self.step(shift));
                PickerOutcome::Changed
            }
            KeyCode::Char('H') => {
                self.adjust(-self.step(true));
                PickerOutcome::Changed
            }
            KeyCode::Char('L') => {
                self.adjust(self.step(true));
                PickerOutcome::Changed
            }
            KeyCode::Home => {
                self.set_component_bound(false);
                PickerOutcome::Changed
            }
            KeyCode::End => {
                self.set_component_bound(true);
                PickerOutcome::Changed
            }
            KeyCode::Char('p') => {
                self.palette = Some(PaletteState::default());
                PickerOutcome::None
            }
            KeyCode::Char(' ') => {
                self.preview_original = !self.preview_original;
                PickerOutcome::Changed
            }
            KeyCode::Char('r') => {
                self.rgb = color_to_rgb(self.default_color);
                self.preview_original = false;
                PickerOutcome::Reset
            }
            KeyCode::Enter => PickerOutcome::Apply,
            KeyCode::Esc => PickerOutcome::Cancel,
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == '-' || ch == '.' => {
                let mut input = Input::default();
                input.handle_event(&Event::Key(key));
                self.input = Some(input);
                PickerOutcome::None
            }
            _ => PickerOutcome::None,
        }
    }

    fn step(&self, big: bool) -> f64 {
        match self.mode {
            ColorMode::Rgb => {
                if big {
                    16.0
                } else {
                    1.0
                }
            }
            ColorMode::Hsv | ColorMode::Hsl => {
                if big {
                    10.0
                } else {
                    1.0
                }
            }
        }
    }

    fn adjust(&mut self, delta: f64) {
        let values = self.component_values();
        let ranges = self.component_ranges();
        let index = self.component.min(2);
        let value = (values[index] + delta).clamp(ranges[index].0, ranges[index].1);
        self.set_component_value(index, value);
    }

    fn set_component_bound(&mut self, maximum: bool) {
        let ranges = self.component_ranges();
        let index = self.component.min(2);
        let value = if maximum {
            ranges[index].1
        } else {
            ranges[index].0
        };
        self.set_component_value(index, value);
    }

    pub fn set_component_value(&mut self, index: usize, value: f64) {
        let index = index.min(2);
        match self.mode {
            ColorMode::Rgb => {
                let mut rgb = self.rgb;
                let channel = value.round().clamp(0.0, 255.0) as u8;
                match index {
                    0 => rgb.0 = channel,
                    1 => rgb.1 = channel,
                    _ => rgb.2 = channel,
                }
                self.rgb = rgb;
            }
            ColorMode::Hsv => {
                let (hue, saturation, hsv_value) = rgb_to_hsv(self.rgb.0, self.rgb.1, self.rgb.2);
                let mut values = [hue, saturation * 100.0, hsv_value * 100.0];
                values[index] = value.clamp(0.0, if index == 0 { 360.0 } else { 100.0 });
                self.rgb = hsv_to_rgb(
                    values[0],
                    (values[1] / 100.0).clamp(0.0, 1.0),
                    (values[2] / 100.0).clamp(0.0, 1.0),
                );
            }
            ColorMode::Hsl => {
                let (hue, saturation, lightness) = rgb_to_hsl(self.rgb.0, self.rgb.1, self.rgb.2);
                let mut values = [hue, saturation * 100.0, lightness * 100.0];
                values[index] = value.clamp(0.0, if index == 0 { 360.0 } else { 100.0 });
                self.rgb = hsl_to_rgb(
                    values[0],
                    (values[1] / 100.0).clamp(0.0, 1.0),
                    (values[2] / 100.0).clamp(0.0, 1.0),
                );
            }
        }
        self.preview_original = false;
    }
}

pub fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Reset => (200, 200, 200),
        Color::Black => (0, 0, 0),
        Color::Red => (128, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (128, 128, 0),
        Color::Blue => (0, 0, 128),
        Color::Magenta => (128, 0, 128),
        Color::Cyan => (0, 128, 128),
        Color::Gray => (192, 192, 192),
        Color::DarkGray => (128, 128, 128),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (0, 0, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Indexed(index) => (index, index, index),
        Color::Rgb(red, green, blue) => (red, green, blue),
    }
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (0, 0, 0);
    }
    let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (red, green, blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_palette_parses_hex_values() {
        assert_eq!(hex_to_rgb("#00aaff"), (0, 170, 255));
        assert_eq!(hex_to_rgb("#000000"), (0, 0, 0));
    }

    #[test]
    fn palette_filters_by_name() {
        let mut palette = PaletteState::default();
        palette.filter = Input::new("green".to_string());
        let filtered = palette.filtered();
        assert!(filtered.iter().any(|(name, _)| *name == "green"));
        assert!(filtered.iter().all(|(name, _)| name.contains("green")));
    }

    #[test]
    fn picker_adjusts_rgb_channels() {
        let mut picker = ColorPicker::new(
            ThemeRole::Accent,
            Color::Rgb(10, 20, 30),
            Color::Rgb(0, 0, 0),
        );
        picker.component = 0;
        picker.adjust(5.0);
        assert_eq!(picker.rgb.0, 15);
        picker.component = 2;
        picker.adjust(-100.0);
        assert_eq!(picker.rgb.2, 0);
    }

    #[test]
    fn picker_switches_modes_and_resets() {
        let mut picker = ColorPicker::new(
            ThemeRole::Error,
            Color::Rgb(10, 20, 30),
            Color::Rgb(1, 2, 3),
        );
        assert_eq!(picker.mode, ColorMode::Rgb);
        picker.mode = picker.mode.next();
        assert_eq!(picker.mode, ColorMode::Hsv);
        picker.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(picker.rgb, (1, 2, 3));
    }

    #[test]
    fn color_to_rgb_maps_reset_to_gray() {
        assert_eq!(color_to_rgb(Color::Reset), (200, 200, 200));
        assert_eq!(color_to_rgb(Color::Rgb(1, 2, 3)), (1, 2, 3));
    }
}
