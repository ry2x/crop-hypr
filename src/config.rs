use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{AppError, Result};

// ── RGBA color ────────────────────────────────────────────────────────────────

/// An RGBA color stored internally as `[red, green, blue, alpha]` floats in
/// `[0.0, 1.0]`.
///
/// In TOML / config files the color is expressed as a CSS-style hex string:
///
/// | Format       | Example       | Alpha         |
/// |------------- |-------------- |-------------- |
/// | `"#RRGGBBAA"` | `"#4585FF8C"` | from last two digits |
/// | `"#RRGGBB"`   | `"#4585FF"`   | `FF` (fully opaque) |
/// | `"#RGBA"`     | `"#458F"`     | short form, each digit doubled |
/// | `"#RGB"`      | `"#45F"`      | short form, alpha = `FF` |
///
/// Parsing is case-insensitive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaColor(pub [f32; 4]);

impl RgbaColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }
}

impl Serialize for RgbaColor {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let [r, g, b, a] = self.0;
        let to_u8 = |f: f32| (f.clamp(0.0, 1.0) * 255.0).round() as u8;
        s.serialize_str(&format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            to_u8(r),
            to_u8(g),
            to_u8(b),
            to_u8(a)
        ))
    }
}

impl<'de> Deserialize<'de> for RgbaColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_hex_color(&s).map_err(de::Error::custom)
    }
}

fn parse_hex_color(s: &str) -> std::result::Result<RgbaColor, String> {
    let s = s.trim();
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| format!("color must start with '#', got {s:?}"))?;

    let from_u8 = |b: u8| b as f32 / 255.0;
    // Expand a single nibble to a full byte: 0xA → 0xAA
    let expand = |h: u8| h << 4 | h;

    let digits: Vec<u8> = hex
        .chars()
        .map(|c| {
            c.to_digit(16)
                .map(|d| d as u8)
                .ok_or_else(|| format!("invalid hex digit {c:?} in {s:?}"))
        })
        .collect::<std::result::Result<_, _>>()?;

    let [r, g, b, a] = match digits.as_slice() {
        [r, g, b] => [expand(*r), expand(*g), expand(*b), 0xFF],
        [r, g, b, a] => [expand(*r), expand(*g), expand(*b), expand(*a)],
        [r1, r2, g1, g2, b1, b2] => [r1 << 4 | r2, g1 << 4 | g2, b1 << 4 | b2, 0xFF],
        [r1, r2, g1, g2, b1, b2, a1, a2] => {
            [r1 << 4 | r2, g1 << 4 | g2, b1 << 4 | b2, a1 << 4 | a2]
        }
        _ => {
            return Err(format!(
                "expected #RGB, #RGBA, #RRGGBB, or #RRGGBBAA, got {s:?}"
            ));
        }
    };

    Ok(RgbaColor::new(
        from_u8(r),
        from_u8(g),
        from_u8(b),
        from_u8(a),
    ))
}

// ── Freeze mode color config ──────────────────────────────────────────────────

/// Semi-transparent black overlay drawn over the frozen screen image.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OverlayColors {
    #[serde(default = "default_overlay_background")]
    pub background: RgbaColor,
}
fn default_overlay_background() -> RgbaColor {
    RgbaColor::new(0.0, 0.0, 0.0, 0.35)
}
impl Default for OverlayColors {
    fn default() -> Self {
        Self {
            background: default_overlay_background(),
        }
    }
}

/// Background pill that contains the mode buttons and cancel button.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToolbarColors {
    #[serde(default = "default_toolbar_background")]
    pub background: RgbaColor,
}
fn default_toolbar_background() -> RgbaColor {
    RgbaColor::new(0.08, 0.08, 0.08, 0.85)
}
impl Default for ToolbarColors {
    fn default() -> Self {
        Self {
            background: default_toolbar_background(),
        }
    }
}

/// Crop / Window / Monitor / All mode buttons.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ButtonColors {
    /// Unselected button background.
    #[serde(default = "default_btn_idle_bg")]
    pub idle_background: RgbaColor,
    /// Unselected button text.
    #[serde(default = "default_btn_idle_text")]
    pub idle_text: RgbaColor,
    /// Selected (active) button background.
    #[serde(default = "default_btn_active_bg")]
    pub active_background: RgbaColor,
    /// Selected button text.
    #[serde(default = "default_btn_active_text")]
    pub active_text: RgbaColor,
    /// Hovered button background (both selected and unselected).
    #[serde(default = "default_btn_hover_bg")]
    pub hover_background: RgbaColor,
    /// Hovered button text.
    #[serde(default = "default_btn_hover_text")]
    pub hover_text: RgbaColor,
}
fn default_btn_idle_bg() -> RgbaColor {
    // Matches iced Dark theme secondary.base (~#797A7D)
    RgbaColor::new(0.475, 0.481, 0.494, 1.0)
}
fn default_btn_idle_text() -> RgbaColor {
    RgbaColor::new(0.90, 0.90, 0.90, 1.0)
}
fn default_btn_active_bg() -> RgbaColor {
    RgbaColor::new(0.345, 0.396, 0.949, 1.0)
}
fn default_btn_active_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
fn default_btn_hover_bg() -> RgbaColor {
    RgbaColor::new(0.42, 0.475, 0.961, 1.0)
}
fn default_btn_hover_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
impl Default for ButtonColors {
    fn default() -> Self {
        Self {
            idle_background: default_btn_idle_bg(),
            idle_text: default_btn_idle_text(),
            active_background: default_btn_active_bg(),
            active_text: default_btn_active_text(),
            hover_background: default_btn_hover_bg(),
            hover_text: default_btn_hover_text(),
        }
    }
}

/// Cancel / close button in the toolbar.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CancelButtonColors {
    #[serde(default = "default_cancel_idle_bg")]
    pub idle_background: RgbaColor,
    #[serde(default = "default_cancel_idle_text")]
    pub idle_text: RgbaColor,
    #[serde(default = "default_cancel_hover_bg")]
    pub hover_background: RgbaColor,
    #[serde(default = "default_cancel_hover_text")]
    pub hover_text: RgbaColor,
}
fn default_cancel_idle_bg() -> RgbaColor {
    RgbaColor::new(0.765, 0.259, 0.247, 1.0)
}
fn default_cancel_idle_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
fn default_cancel_hover_bg() -> RgbaColor {
    RgbaColor::new(0.831, 0.290, 0.278, 1.0)
}
fn default_cancel_hover_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
impl Default for CancelButtonColors {
    fn default() -> Self {
        Self {
            idle_background: default_cancel_idle_bg(),
            idle_text: default_cancel_idle_text(),
            hover_background: default_cancel_hover_bg(),
            hover_text: default_cancel_hover_text(),
        }
    }
}

/// Highlight frame drawn over windows in Window-selection mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowFrameColors {
    #[serde(default = "default_wf_fill_idle")]
    pub fill_idle: RgbaColor,
    #[serde(default = "default_wf_fill_hovered")]
    pub fill_hovered: RgbaColor,
    #[serde(default = "default_wf_stroke_idle")]
    pub stroke_idle: RgbaColor,
    #[serde(default = "default_wf_stroke_hovered")]
    pub stroke_hovered: RgbaColor,
    /// Window title text shown when hovered.
    #[serde(default = "default_wf_label_text")]
    pub label_text: RgbaColor,
    /// "Click to capture" hint shown when hovered.
    #[serde(default = "default_wf_hint_text")]
    pub hint_text: RgbaColor,
}
fn default_wf_fill_idle() -> RgbaColor {
    RgbaColor::new(0.27, 0.52, 1.0, 0.20)
}
fn default_wf_fill_hovered() -> RgbaColor {
    RgbaColor::new(0.27, 0.52, 1.0, 0.55)
}
fn default_wf_stroke_idle() -> RgbaColor {
    RgbaColor::new(0.3, 0.6, 1.0, 0.70)
}
fn default_wf_stroke_hovered() -> RgbaColor {
    RgbaColor::new(0.3, 0.6, 1.0, 1.0)
}
fn default_wf_label_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
fn default_wf_hint_text() -> RgbaColor {
    RgbaColor::new(0.8, 0.9, 1.0, 0.9)
}
impl Default for WindowFrameColors {
    fn default() -> Self {
        Self {
            fill_idle: default_wf_fill_idle(),
            fill_hovered: default_wf_fill_hovered(),
            stroke_idle: default_wf_stroke_idle(),
            stroke_hovered: default_wf_stroke_hovered(),
            label_text: default_wf_label_text(),
            hint_text: default_wf_hint_text(),
        }
    }
}

/// Highlight frame drawn over monitors in Monitor-selection mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MonitorFrameColors {
    #[serde(default = "default_mf_fill_idle")]
    pub fill_idle: RgbaColor,
    #[serde(default = "default_mf_fill_hovered")]
    pub fill_hovered: RgbaColor,
    #[serde(default = "default_mf_stroke_idle")]
    pub stroke_idle: RgbaColor,
    #[serde(default = "default_mf_stroke_hovered")]
    pub stroke_hovered: RgbaColor,
    /// Monitor name text when hovered.
    #[serde(default = "default_mf_label_text")]
    pub label_text: RgbaColor,
    /// "Click to capture" hint shown when hovered.
    #[serde(default = "default_mf_hint_text")]
    pub hint_text: RgbaColor,
    /// Monitor name text when not hovered (always visible at reduced opacity).
    #[serde(default = "default_mf_name_text_idle")]
    pub name_text_idle: RgbaColor,
}
fn default_mf_fill_idle() -> RgbaColor {
    RgbaColor::new(0.27, 0.52, 1.0, 0.08)
}
fn default_mf_fill_hovered() -> RgbaColor {
    RgbaColor::new(0.27, 0.52, 1.0, 0.40)
}
fn default_mf_stroke_idle() -> RgbaColor {
    RgbaColor::new(0.3, 0.6, 1.0, 0.35)
}
fn default_mf_stroke_hovered() -> RgbaColor {
    RgbaColor::new(0.3, 0.6, 1.0, 1.0)
}
fn default_mf_label_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
fn default_mf_hint_text() -> RgbaColor {
    RgbaColor::new(0.8, 0.9, 1.0, 0.9)
}
fn default_mf_name_text_idle() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 0.5)
}
impl Default for MonitorFrameColors {
    fn default() -> Self {
        Self {
            fill_idle: default_mf_fill_idle(),
            fill_hovered: default_mf_fill_hovered(),
            stroke_idle: default_mf_stroke_idle(),
            stroke_hovered: default_mf_stroke_hovered(),
            label_text: default_mf_label_text(),
            hint_text: default_mf_hint_text(),
            name_text_idle: default_mf_name_text_idle(),
        }
    }
}

/// Rubber-band rectangle and size label in Crop-selection mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CropFrameColors {
    #[serde(default = "default_crop_stroke")]
    pub stroke: RgbaColor,
    #[serde(default = "default_crop_label_text")]
    pub label_text: RgbaColor,
}
fn default_crop_stroke() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
fn default_crop_label_text() -> RgbaColor {
    RgbaColor::new(1.0, 1.0, 1.0, 1.0)
}
impl Default for CropFrameColors {
    fn default() -> Self {
        Self {
            stroke: default_crop_stroke(),
            label_text: default_crop_label_text(),
        }
    }
}

/// All user-configurable colors for the freeze-mode overlay UI.
///
/// Every sub-table is optional in TOML; omitted keys fall back to the defaults
/// shown in the sample config.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FreezeColors {
    #[serde(default)]
    pub overlay: OverlayColors,
    #[serde(default)]
    pub toolbar: ToolbarColors,
    #[serde(default)]
    pub button: ButtonColors,
    #[serde(default)]
    pub cancel_button: CancelButtonColors,
    #[serde(default)]
    pub window_frame: WindowFrameColors,
    #[serde(default)]
    pub monitor_frame: MonitorFrameColors,
    #[serde(default)]
    pub crop_frame: CropFrameColors,
}

// ── Freeze UI glyphs ──────────────────────────────────────────────────────────

/// Icon glyphs displayed in the freeze-mode toolbar.
/// Defaults match the Nerd Fonts / Material Design icons used by default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeGlyphs {
    #[serde(default = "default_glyph_crop")]
    pub crop: String,
    #[serde(default = "default_glyph_window")]
    pub window: String,
    #[serde(default = "default_glyph_monitor")]
    pub monitor: String,
    #[serde(default = "default_glyph_all")]
    pub all: String,
    #[serde(default = "default_glyph_cancel")]
    pub cancel: String,
}

fn default_glyph_crop() -> String {
    "\u{F019F}".to_string()
}
fn default_glyph_window() -> String {
    "\u{EB7F}".to_string()
}
fn default_glyph_monitor() -> String {
    "\u{F0379}".to_string()
}
fn default_glyph_all() -> String {
    "\u{F004C}".to_string()
}
fn default_glyph_cancel() -> String {
    "\u{F05AD}".to_string()
}

impl Default for FreezeGlyphs {
    fn default() -> Self {
        Self {
            crop: default_glyph_crop(),
            window: default_glyph_window(),
            monitor: default_glyph_monitor(),
            all: default_glyph_all(),
            cancel: default_glyph_cancel(),
        }
    }
}

// ── Toolbar position ──────────────────────────────────────────────────────────

/// Edge of the screen where the freeze-mode toolbar is docked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolbarPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_save_path")]
    pub save_path: PathBuf,

    #[serde(default = "default_filename_pattern")]
    pub filename_pattern: String,

    #[serde(default)]
    pub freeze_glyphs: FreezeGlyphs,

    #[serde(default)]
    pub toolbar_position: ToolbarPosition,

    /// When `true`, window captures include the Hyprland border (expanded by
    /// `general:border_size` on each side) and the freeze-mode overlay draws
    /// rounded highlight frames matching `decoration:rounding`.
    #[serde(default = "default_capture_window_border")]
    pub capture_window_border: bool,

    /// Colors for every element of the freeze-mode overlay UI.
    /// All keys are optional; omitted keys fall back to the built-in defaults.
    #[serde(default)]
    pub freeze_colors: FreezeColors,
}

fn default_capture_window_border() -> bool {
    false
}

fn default_save_path() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("Screenshots")
}

fn default_filename_pattern() -> String {
    "hyprsnap_%Y%m%d_%H%M%S".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_path: default_save_path(),
            filename_pattern: default_filename_pattern(),
            freeze_glyphs: FreezeGlyphs::default(),
            toolbar_position: ToolbarPosition::default(),
            capture_window_border: default_capture_window_border(),
            freeze_colors: FreezeColors::default(),
        }
    }
}

impl Config {
    /// Load config from the default path (`~/.config/crop-hypr/config.toml`).
    /// Falls back to defaults if the file does not exist.
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_config_path())
    }

    /// Returns the default config file path (`~/.config/crop-hypr/config.toml`).
    pub fn default_config_path() -> PathBuf {
        default_config_path()
    }

    /// Load config from an explicit path.
    /// Falls back to defaults if the file does not exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        let mut cfg = if !path.exists() {
            Self::default()
        } else {
            let raw = fs::read_to_string(path)
                .map_err(|e| AppError::FileSystem(path.to_path_buf(), e))?;
            toml::from_str(&raw)?
        };

        cfg.save_path = expand_tilde(&cfg.save_path);
        cfg.validate()?;

        Ok(cfg)
    }

    /// Serialize the default config to a TOML string, suitable for writing to
    /// a config file. Used by the `generate-config` command.
    pub fn generate_default_toml() -> Result<String> {
        toml::to_string_pretty(&Self::default())
            .map_err(|e| AppError::Config(format!("failed to serialize default config: {e}")))
    }

    fn validate(&self) -> Result<()> {
        if self.filename_pattern.trim().is_empty() {
            return Err(AppError::Config(
                "filename_pattern cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn output_filename(&self) -> String {
        let ts = chrono::Local::now()
            .format(&self.filename_pattern)
            .to_string();
        format!("{ts}.png")
    }

    pub fn output_path(&self) -> PathBuf {
        self.save_path.join(self.output_filename())
    }
}

fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("crop-hypr")
        .join("config.toml")
}

fn expand_tilde(path: &std::path::Path) -> PathBuf {
    let s = path.to_string_lossy();
    let expanded = if let Some(stripped) = s.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(stripped)
    } else if s == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    } else {
        path.to_path_buf()
    };

    if expanded.is_absolute() {
        expanded
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn write_toml(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        f.write_all(content.as_bytes()).expect("write");
        f
    }

    // ── RgbaColor parsing ─────────────────────────────────────────────────────

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0 / 255.0
    }

    #[test]
    fn test_rgba_rrggbbaa() {
        let c = parse_hex_color("#4585FF8C").unwrap();
        assert!(approx_eq(c.0[0], 0x45 as f32 / 255.0));
        assert!(approx_eq(c.0[1], 0x85 as f32 / 255.0));
        assert!(approx_eq(c.0[2], 0xFF as f32 / 255.0));
        assert!(approx_eq(c.0[3], 0x8C as f32 / 255.0));
    }

    #[test]
    fn test_rgba_rrggbb_alpha_is_ff() {
        let c = parse_hex_color("#4585FF").unwrap();
        assert!(approx_eq(c.0[2], 1.0));
        assert!(approx_eq(c.0[3], 1.0), "alpha should be FF = 1.0");
    }

    #[test]
    fn test_rgba_rgb_shorthand() {
        let c = parse_hex_color("#F80").unwrap();
        assert!(approx_eq(c.0[0], 0xFF as f32 / 255.0));
        assert!(approx_eq(c.0[1], 0x88 as f32 / 255.0));
        assert!(approx_eq(c.0[2], 0x00 as f32 / 255.0));
        assert!(approx_eq(c.0[3], 1.0));
    }

    #[test]
    fn test_rgba_rgba_shorthand() {
        let c = parse_hex_color("#F80A").unwrap();
        assert!(approx_eq(c.0[0], 0xFF as f32 / 255.0));
        assert!(approx_eq(c.0[1], 0x88 as f32 / 255.0));
        assert!(approx_eq(c.0[2], 0x00 as f32 / 255.0));
        assert!(approx_eq(c.0[3], 0xAA as f32 / 255.0));
    }

    #[test]
    fn test_rgba_lowercase() {
        let upper = parse_hex_color("#4585ff8c").unwrap();
        let lower = parse_hex_color("#4585FF8C").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_rgba_missing_hash_is_error() {
        assert!(parse_hex_color("4585FF8C").is_err());
    }

    #[test]
    fn test_rgba_invalid_digit_is_error() {
        assert!(parse_hex_color("#GGGGGGGG").is_err());
    }

    #[test]
    fn test_rgba_wrong_length_is_error() {
        assert!(parse_hex_color("#12345").is_err()); // 5 digits
        assert!(parse_hex_color("#1234567").is_err()); // 7 digits
    }

    #[test]
    fn test_rgba_serialize_round_trip() {
        #[derive(Serialize, Deserialize)]
        struct W {
            c: RgbaColor,
        }

        let original = RgbaColor::new(
            0x45 as f32 / 255.0,
            0x85 as f32 / 255.0,
            1.0,
            0x8C as f32 / 255.0,
        );
        let s = toml::to_string(&W { c: original }).unwrap();
        assert!(s.contains('#'), "serialized form should contain '#': {s}");
        let back: W = toml::from_str(&s).unwrap();
        assert_eq!(original, back.c);
    }

    #[test]
    fn test_freeze_colors_deserialize_from_hex() {
        // Use r##"..."## so that "#RRGGBBAA" inside doesn't close the delimiter
        let toml_str = r##"
[freeze_colors.window_frame]
fill_hovered = "#4585FF8C"
stroke_hovered = "#4D99FFFF"
"##;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        let wf = cfg.freeze_colors.window_frame;
        assert!(approx_eq(wf.fill_hovered.0[3], 0x8C as f32 / 255.0));
        assert!(approx_eq(wf.stroke_hovered.0[3], 1.0));
    }

    #[test]
    fn test_default_colors_round_trip() {
        let s = Config::generate_default_toml().unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        let d = Config::default();
        // Hex format has 8-bit precision; compare with 1/255 tolerance
        let a = back.freeze_colors.window_frame.fill_hovered.0;
        let b = d.freeze_colors.window_frame.fill_hovered.0;
        for i in 0..4 {
            assert!(
                approx_eq(a[i], b[i]),
                "channel {i}: {:.4} != {:.4}",
                a[i],
                b[i]
            );
        }
    }

    #[test]
    fn test_default_config_save_path() {
        let cfg = Config::default();
        assert!(
            cfg.save_path.to_string_lossy().contains("Screenshots"),
            "save_path should contain 'Screenshots'"
        );
    }

    #[test]
    fn test_default_config_filename_pattern() {
        assert_eq!(
            Config::default().filename_pattern,
            default_filename_pattern()
        );
    }

    #[test]
    fn test_default_freeze_glyphs() {
        let g = FreezeGlyphs::default();
        assert_eq!(g.crop, default_glyph_crop());
        assert_eq!(g.window, default_glyph_window());
        assert_eq!(g.monitor, default_glyph_monitor());
        assert_eq!(g.all, default_glyph_all());
        assert_eq!(g.cancel, default_glyph_cancel());
    }

    // ── validation ────────────────────────────────────────────────────────────

    #[test]
    fn test_validation_accepts_non_empty_pattern() {
        let cfg: Config = toml::from_str("filename_pattern = 'test'").unwrap();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validation_rejects_empty_pattern() {
        let cfg: Config = toml::from_str("filename_pattern = ''").unwrap();
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("filename_pattern cannot be empty"));
    }

    #[test]
    fn test_validation_rejects_whitespace_only_pattern() {
        let cfg: Config = toml::from_str("filename_pattern = '   '").unwrap();
        assert!(cfg.validate().is_err());
    }

    // ── freeze_glyphs deserialization ─────────────────────────────────────────

    #[test]
    fn test_freeze_glyphs_partial_override() {
        let cfg: Config = toml::from_str("[freeze_glyphs]\ncrop = \"X\"").unwrap();
        assert_eq!(cfg.freeze_glyphs.crop, "X");
        // All unspecified fields fall back to defaults.
        assert_eq!(cfg.freeze_glyphs.window, default_glyph_window());
        assert_eq!(cfg.freeze_glyphs.monitor, default_glyph_monitor());
        assert_eq!(cfg.freeze_glyphs.all, default_glyph_all());
        assert_eq!(cfg.freeze_glyphs.cancel, default_glyph_cancel());
    }

    #[test]
    fn test_freeze_glyphs_full_override() {
        let toml = r#"
[freeze_glyphs]
crop    = "A"
window  = "B"
monitor = "C"
all     = "D"
cancel  = "E"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.freeze_glyphs.crop, "A");
        assert_eq!(cfg.freeze_glyphs.window, "B");
        assert_eq!(cfg.freeze_glyphs.monitor, "C");
        assert_eq!(cfg.freeze_glyphs.all, "D");
        assert_eq!(cfg.freeze_glyphs.cancel, "E");
    }

    // ── toolbar_position ──────────────────────────────────────────────────────

    #[test]
    fn test_toolbar_position_default_is_top() {
        assert_eq!(Config::default().toolbar_position, ToolbarPosition::Top);
    }

    #[test]
    fn test_toolbar_position_deserializes_all_variants() {
        for (s, expected) in [
            ("top", ToolbarPosition::Top),
            ("bottom", ToolbarPosition::Bottom),
            ("left", ToolbarPosition::Left),
            ("right", ToolbarPosition::Right),
        ] {
            let toml = format!("toolbar_position = \"{s}\"");
            let cfg: Config = toml::from_str(&toml).unwrap();
            assert_eq!(cfg.toolbar_position, expected, "failed for {s}");
        }
    }

    #[test]
    fn test_toolbar_position_missing_defaults_to_top() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.toolbar_position, ToolbarPosition::Top);
    }

    // ── generate_default_toml ─────────────────────────────────────────────────

    #[test]
    fn test_generate_default_toml_is_valid_toml() {
        let s = Config::generate_default_toml().expect("serialize");
        toml::from_str::<Config>(&s).expect("generated TOML must parse cleanly");
    }

    #[test]
    fn test_generate_default_toml_round_trips_all_fields() {
        let original = Config::default();
        let parsed: Config =
            toml::from_str(&Config::generate_default_toml().expect("serialize")).expect("parse");

        assert_eq!(parsed.filename_pattern, original.filename_pattern);
        assert_eq!(parsed.freeze_glyphs.crop, original.freeze_glyphs.crop);
        assert_eq!(parsed.freeze_glyphs.window, original.freeze_glyphs.window);
        assert_eq!(parsed.freeze_glyphs.monitor, original.freeze_glyphs.monitor);
        assert_eq!(parsed.freeze_glyphs.all, original.freeze_glyphs.all);
        assert_eq!(parsed.freeze_glyphs.cancel, original.freeze_glyphs.cancel);
        assert_eq!(parsed.toolbar_position, original.toolbar_position);
    }

    // ── load_from ─────────────────────────────────────────────────────────────

    #[test]
    fn test_load_from_nonexistent_returns_defaults() {
        let cfg = Config::load_from(std::path::Path::new("/nonexistent/path/config.toml"))
            .expect("missing file should yield defaults");
        assert_eq!(cfg.filename_pattern, default_filename_pattern());
    }

    #[test]
    fn test_load_from_file_overrides_fields() {
        let f = write_toml(
            r#"
filename_pattern = "snap_%Y"
[freeze_glyphs]
cancel = "Z"
"#,
        );
        let cfg = Config::load_from(f.path()).expect("load");
        assert_eq!(cfg.filename_pattern, "snap_%Y");
        assert_eq!(cfg.freeze_glyphs.cancel, "Z");
        // Unspecified fields still default.
        assert_eq!(cfg.freeze_glyphs.crop, default_glyph_crop());
    }

    #[test]
    fn test_load_from_invalid_toml_returns_error() {
        let f = write_toml("not valid toml [[[");
        assert!(Config::load_from(f.path()).is_err());
    }

    // ── output helpers ────────────────────────────────────────────────────────

    #[test]
    fn test_output_filename_has_png_extension() {
        assert!(Config::default().output_filename().ends_with(".png"));
    }

    #[test]
    fn test_output_path_is_under_save_path() {
        let cfg = Config::default();
        let path = cfg.output_path();
        assert_eq!(path.parent().unwrap(), cfg.save_path);
    }

    // ── tilde expansion ───────────────────────────────────────────────────────

    #[test]
    fn test_tilde_expansion() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        assert_eq!(
            expand_tilde(&PathBuf::from("~/test/dir")),
            home.join("test/dir")
        );
        assert_eq!(expand_tilde(&PathBuf::from("~")), home);
        assert_eq!(
            expand_tilde(&PathBuf::from("/tmp/test")),
            PathBuf::from("/tmp/test")
        );
        // Bare relative path is anchored to $HOME.
        assert_eq!(
            expand_tilde(&PathBuf::from("Screenshots")),
            home.join("Screenshots")
        );
    }
}
