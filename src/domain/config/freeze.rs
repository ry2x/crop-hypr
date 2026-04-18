use serde::{Deserialize, Serialize};

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
    /// Text size (in pixels) of the glyph inside each toolbar button.
    #[serde(default = "default_glyph_size")]
    pub size: f32,
}

pub(crate) fn default_glyph_crop() -> String {
    "\u{F019F}".to_string()
}
pub(crate) fn default_glyph_window() -> String {
    "\u{EB7F}".to_string()
}
pub(crate) fn default_glyph_monitor() -> String {
    "\u{F0379}".to_string()
}
pub(crate) fn default_glyph_all() -> String {
    "\u{F004C}".to_string()
}
pub(crate) fn default_glyph_cancel() -> String {
    "\u{F05AD}".to_string()
}
pub(crate) fn default_glyph_size() -> f32 {
    26.0
}

impl Default for FreezeGlyphs {
    fn default() -> Self {
        Self {
            crop: default_glyph_crop(),
            window: default_glyph_window(),
            monitor: default_glyph_monitor(),
            all: default_glyph_all(),
            cancel: default_glyph_cancel(),
            size: default_glyph_size(),
        }
    }
}

// ── Toolbar button visibility ─────────────────────────────────────────────────

/// Controls which buttons are visible in the freeze-mode toolbar.
/// Buttons set to `false` are omitted entirely; if all are `false`, the
/// toolbar container is not rendered.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FreezeButtons {
    #[serde(default = "default_true")]
    pub crop: bool,
    #[serde(default = "default_true")]
    pub window: bool,
    #[serde(default = "default_true")]
    pub monitor: bool,
    #[serde(default = "default_true")]
    pub all: bool,
    #[serde(default = "default_true")]
    pub cancel: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FreezeButtons {
    fn default() -> Self {
        Self {
            crop: true,
            window: true,
            monitor: true,
            all: true,
            cancel: true,
        }
    }
}

impl FreezeButtons {
    /// Returns `true` if at least one button (including cancel) is enabled.
    pub fn any_visible(&self) -> bool {
        self.crop || self.window || self.monitor || self.all || self.cancel
    }

    /// Returns `true` if at least one *capture-mode* button is enabled.
    ///
    /// When this returns `false` (all of crop/window/monitor/all are disabled),
    /// freeze mode falls back to `Crop` canvas selection so the user can still
    /// drag-select a region even without toolbar buttons. The `cancel` button is
    /// excluded because it does not initiate a capture.
    pub fn any_capture_enabled(&self) -> bool {
        self.crop || self.window || self.monitor || self.all
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
