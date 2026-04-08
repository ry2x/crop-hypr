use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

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

pub(super) fn parse_hex_color(s: &str) -> std::result::Result<RgbaColor, String> {
    let s = s.trim();
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| format!("color must start with '#', got {s:?}"))?;

    let from_u8 = |b: u8| b as f32 / 255.0;
    // Expand a single nibble to a full byte: 0xA → 0xAA
    let expand = |h: u8| h << 4 | h;

    let mut digits: Vec<u8> = Vec::with_capacity(8);
    for c in hex.chars() {
        digits.push(
            c.to_digit(16)
                .map(|d| d as u8)
                .ok_or_else(|| format!("invalid hex digit {c:?} in {s:?}"))?,
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0 / 255.0
    }

    // ── RgbaColor parsing ─────────────────────────────────────────────────────

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

    // ── FreezeColors ─────────────────────────────────────────────────────────

    #[test]
    fn test_freeze_colors_deserialize_from_hex() {
        let toml_str = r##"
[window_frame]
fill_hovered = "#4585FF8C"
stroke_hovered = "#4D99FFFF"
"##;
        let fc: FreezeColors = toml::from_str(toml_str).unwrap();
        let wf = fc.window_frame;
        assert!(approx_eq(wf.fill_hovered.0[3], 0x8C as f32 / 255.0));
        assert!(approx_eq(wf.stroke_hovered.0[3], 1.0));
    }

    #[test]
    fn test_default_colors_round_trip() {
        let original = FreezeColors::default();
        let s = toml::to_string_pretty(&original).unwrap();
        let back: FreezeColors = toml::from_str(&s).unwrap();
        // Hex format has 8-bit precision; compare with 1/255 tolerance
        let a = back.window_frame.fill_hovered.0;
        let b = original.window_frame.fill_hovered.0;
        for i in 0..4 {
            assert!(
                approx_eq(a[i], b[i]),
                "channel {i}: {:.4} != {:.4}",
                a[i],
                b[i]
            );
        }
    }
}
