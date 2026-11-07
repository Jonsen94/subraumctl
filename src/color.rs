//! Theme roles, color parsing and RGB/HSV/HSL conversions.
//!
//! Color arguments accept CSS names, `#hex`, `rgb(...)`, `hsl(...)`, bare
//! `r,g,b` triplets and bare hex strings. Alpha is parsed but ignored, because
//! terminal colors have no alpha channel.

use clap::ValueEnum;
use csscolorparser::Color as CssColor;
use ratatui::style::Color;
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ThemePreset {
    #[value(name = "dark")]
    Dark,
    #[value(name = "light")]
    Light,
    #[value(name = "high-contrast", alias = "highcontrast")]
    HighContrast,
}

impl ThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::HighContrast => "high-contrast",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThemeRole {
    Background,
    Foreground,
    Accent,
    Selection,
    Success,
    Warning,
    Error,
    Muted,
    Border,
    Alert,
}

impl ThemeRole {
    pub const ALL: [Self; 10] = [
        Self::Background,
        Self::Foreground,
        Self::Accent,
        Self::Selection,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Muted,
        Self::Border,
        Self::Alert,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Foreground => "foreground",
            Self::Accent => "accent",
            Self::Selection => "selection",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Muted => "muted",
            Self::Border => "border",
            Self::Alert => "alert",
        }
    }

    pub fn cli_flag(self) -> &'static str {
        match self {
            Self::Background => "--background-color",
            Self::Foreground => "--foreground-color",
            Self::Accent => "--accent-color",
            Self::Selection => "--selection-color",
            Self::Success => "--success-color",
            Self::Warning => "--warning-color",
            Self::Error => "--error-color",
            Self::Muted => "--muted-color",
            Self::Border => "--border-color",
            Self::Alert => "--alert-color",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub selection: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub border: Color,
    pub alert: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::preset(ThemePreset::Dark)
    }
}

impl Theme {
    pub fn preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Dark => Self {
                background: Color::Reset,
                foreground: Color::Reset,
                accent: Color::Rgb(0, 200, 200),
                selection: Color::Rgb(30, 60, 90),
                success: Color::Rgb(80, 200, 80),
                warning: Color::Rgb(230, 190, 60),
                error: Color::Rgb(230, 80, 80),
                muted: Color::Rgb(120, 120, 120),
                border: Color::Rgb(70, 70, 70),
                alert: Color::Rgb(230, 90, 230),
            },
            ThemePreset::Light => Self {
                background: Color::Reset,
                foreground: Color::Reset,
                accent: Color::Rgb(0, 110, 140),
                selection: Color::Rgb(180, 210, 240),
                success: Color::Rgb(0, 120, 0),
                warning: Color::Rgb(160, 110, 0),
                error: Color::Rgb(180, 0, 0),
                muted: Color::Rgb(100, 100, 100),
                border: Color::Rgb(150, 150, 150),
                alert: Color::Rgb(150, 0, 150),
            },
            ThemePreset::HighContrast => Self {
                background: Color::Black,
                foreground: Color::White,
                accent: Color::Yellow,
                selection: Color::Blue,
                success: Color::LightGreen,
                warning: Color::LightYellow,
                error: Color::LightRed,
                muted: Color::Gray,
                border: Color::White,
                alert: Color::Magenta,
            },
        }
    }

    pub fn get(&self, role: ThemeRole) -> Color {
        match role {
            ThemeRole::Background => self.background,
            ThemeRole::Foreground => self.foreground,
            ThemeRole::Accent => self.accent,
            ThemeRole::Selection => self.selection,
            ThemeRole::Success => self.success,
            ThemeRole::Warning => self.warning,
            ThemeRole::Error => self.error,
            ThemeRole::Muted => self.muted,
            ThemeRole::Border => self.border,
            ThemeRole::Alert => self.alert,
        }
    }

    pub fn set(&mut self, role: ThemeRole, color: Color) {
        match role {
            ThemeRole::Background => self.background = color,
            ThemeRole::Foreground => self.foreground = color,
            ThemeRole::Accent => self.accent = color,
            ThemeRole::Selection => self.selection = color,
            ThemeRole::Success => self.success = color,
            ThemeRole::Warning => self.warning = color,
            ThemeRole::Error => self.error = color,
            ThemeRole::Muted => self.muted = color,
            ThemeRole::Border => self.border = color,
            ThemeRole::Alert => self.alert = color,
        }
    }

    pub fn reset_role(&mut self, role: ThemeRole, preset: ThemePreset) {
        self.set(role, Self::preset(preset).get(role));
    }
}

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("invalid color '{0}'")]
    Invalid(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParsedColor {
    pub color: Color,
    pub alpha_ignored: bool,
}

pub fn parse_color(input: &str) -> Result<ParsedColor, ColorError> {
    let candidate = normalize_color(input);
    let parsed: CssColor = candidate
        .parse()
        .map_err(|_| ColorError::Invalid(input.to_string()))?;
    let [red, green, blue, alpha] = parsed.to_rgba8();
    let color = Color::Rgb(red, green, blue);
    Ok(ParsedColor {
        color,
        alpha_ignored: alpha < 255,
    })
}

fn normalize_color(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with('#')
        || lower.starts_with("rgb")
        || lower.starts_with("hsl")
        || lower.starts_with("hsv")
    {
        return trimmed.to_string();
    }

    let looks_like_triplet = trimmed.contains(',')
        && trimmed.chars().all(|ch| {
            ch.is_ascii_digit() || ch == ',' || ch == '.' || ch == ' ' || ch == '%' || ch == '-'
        });
    if looks_like_triplet {
        return format!("rgb({trimmed})");
    }

    let looks_like_hex =
        trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) && matches!(trimmed.len(), 3 | 4 | 6 | 8);
    if looks_like_hex {
        return format!("#{trimmed}");
    }

    trimmed.to_string()
}

pub fn color_to_hex(color: Color) -> String {
    match color {
        Color::Reset => "default".to_string(),
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Gray => "gray".to_string(),
        Color::DarkGray => "darkgray".to_string(),
        Color::LightRed => "lightred".to_string(),
        Color::LightGreen => "lightgreen".to_string(),
        Color::LightYellow => "lightyellow".to_string(),
        Color::LightBlue => "lightblue".to_string(),
        Color::LightMagenta => "lightmagenta".to_string(),
        Color::LightCyan => "lightcyan".to_string(),
        Color::White => "white".to_string(),
        Color::Indexed(index) => format!("indexed({index})"),
        Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
    }
}

pub fn rgb_to_hsv(red: u8, green: u8, blue: u8) -> (f64, f64, f64) {
    let r = red as f64 / 255.0;
    let g = green as f64 / 255.0;
    let b = blue as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta.abs() < f64::EPSILON {
        0.0
    } else if (max - r).abs() < f64::EPSILON {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    let saturation = if max.abs() < f64::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, saturation, max)
}

pub fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> (u8, u8, u8) {
    let hue = hue.rem_euclid(360.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    let chroma = value * saturation;
    let x = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = value - chroma;
    let (r, g, b) = match hue as u32 / 60 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    (
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

pub fn rgb_to_hsl(red: u8, green: u8, blue: u8) -> (f64, f64, f64) {
    let r = red as f64 / 255.0;
    let g = green as f64 / 255.0;
    let b = blue as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let delta = max - min;
    let saturation = if delta.abs() < f64::EPSILON {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };

    let hue = if delta.abs() < f64::EPSILON {
        0.0
    } else if (max - r).abs() < f64::EPSILON {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    (hue, saturation, lightness)
}

pub fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
    let hue = hue.rem_euclid(360.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let lightness = lightness.clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let x = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = lightness - chroma / 2.0;
    let (r, g, b) = match hue as u32 / 60 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    (
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_hex_and_functions() {
        assert_eq!(parse_color("red").unwrap().color, Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#0f0").unwrap().color, Color::Rgb(0, 255, 0));
        assert_eq!(
            parse_color("rgb(0, 128, 128)").unwrap().color,
            Color::Rgb(0, 128, 128)
        );
        assert_eq!(
            parse_color("0,128,128").unwrap().color,
            Color::Rgb(0, 128, 128)
        );
        assert_eq!(
            parse_color("00aaff").unwrap().color,
            Color::Rgb(0, 170, 255)
        );
        assert_eq!(parse_color("teal").unwrap().color, Color::Rgb(0, 128, 128));
    }

    #[test]
    fn alpha_is_flagged_and_ignored() {
        let parsed = parse_color("rgba(1, 2, 3, 0.5)").unwrap();
        assert!(parsed.alpha_ignored);
        assert_eq!(parsed.color, Color::Rgb(1, 2, 3));
    }

    #[test]
    fn invalid_colors_are_rejected() {
        assert!(parse_color("definitely-not-a-color").is_err());
        assert!(parse_color("").is_err());
    }

    #[test]
    fn hsv_round_trips_primary_colors() {
        for (rgb, hsv) in [
            ((255, 0, 0), (0.0, 1.0, 1.0)),
            ((0, 255, 0), (120.0, 1.0, 1.0)),
            ((0, 0, 255), (240.0, 1.0, 1.0)),
        ] {
            let converted = rgb_to_hsv(rgb.0, rgb.1, rgb.2);
            assert!((converted.0 - hsv.0).abs() < 1e-6);
            assert!((converted.1 - hsv.1).abs() < 1e-6);
            assert!((converted.2 - hsv.2).abs() < 1e-6);
            assert_eq!(hsv_to_rgb(converted.0, converted.1, converted.2), rgb);
        }
    }

    #[test]
    fn hsl_round_trips_primary_colors() {
        for (rgb, hsl) in [
            ((255, 0, 0), (0.0, 1.0, 0.5)),
            ((0, 255, 0), (120.0, 1.0, 0.5)),
            ((0, 0, 255), (240.0, 1.0, 0.5)),
        ] {
            let converted = rgb_to_hsl(rgb.0, rgb.1, rgb.2);
            assert!((converted.0 - hsl.0).abs() < 1e-6);
            assert!((converted.1 - hsl.1).abs() < 1e-6);
            assert!((converted.2 - hsl.2).abs() < 1e-6);
            assert_eq!(hsl_to_rgb(converted.0, converted.1, converted.2), rgb);
        }
    }

    #[test]
    fn theme_presets_are_distinct_and_roles_round_trip() {
        let dark = Theme::preset(ThemePreset::Dark);
        let light = Theme::preset(ThemePreset::Light);
        assert_ne!(dark.accent, light.accent);
        for role in ThemeRole::ALL {
            let mut theme = dark.clone();
            theme.set(role, Color::Rgb(1, 2, 3));
            assert_eq!(theme.get(role), Color::Rgb(1, 2, 3));
        }
    }

    #[test]
    fn hex_formatting() {
        assert_eq!(color_to_hex(Color::Rgb(0, 170, 255)), "#00aaff");
        assert_eq!(color_to_hex(Color::Reset), "default");
    }
}
