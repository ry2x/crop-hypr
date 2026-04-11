pub mod colors;
pub use colors::{
    ButtonColors, CancelButtonColors, CropFrameColors, FreezeColors, MonitorFrameColors,
    OverlayColors, RgbaColor, ToolbarColors, WindowFrameColors,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

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
    /// Load config from the default path (`~/.config/hyprcrop/config.toml`).
    /// Falls back to defaults if the file does not exist.
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_config_path())
    }

    /// Returns the default config file path (`~/.config/hyprcrop/config.toml`).
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
        .join("hyprcrop")
        .join("config.toml")
}

fn expand_tilde(path: &std::path::Path) -> PathBuf {
    let s = path.to_string_lossy();
    let (expanded, from_tilde_or_relative) = if let Some(stripped) = s.strip_prefix("~/") {
        (
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(stripped),
            true,
        )
    } else if s == "~" {
        (
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
            true,
        )
    } else {
        (path.to_path_buf(), false)
    };

    let resolved = if expanded.is_absolute() {
        expanded
    } else {
        // Relative paths are anchored to home.
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(expanded)
    };

    let normalized = normalize_path(resolved);

    // For tilde-expanded or relative paths, guard against `..` traversal
    // that escapes the home directory (e.g. `~/foo/../../etc`).
    // Explicit absolute paths (e.g. `/tmp/screenshots`) are passed through as-is.
    if from_tilde_or_relative || !path.is_absolute() {
        if let Some(home) = dirs::home_dir() {
            if !normalized.starts_with(&home) {
                eprintln!(
                    "[hyprcrop] warning: save_path '{}' resolves outside home directory, falling back to ~/Screenshots",
                    path.display()
                );
                return home.join("Screenshots");
            }
        }
    }

    normalized
}

/// Resolve `.` and `..` path components without touching the filesystem.
///
/// Correctly preserves Prefix/RootDir anchors so that e.g. `/../etc` stays
/// as `/etc` (not a relative `etc`). Leading `..` on relative paths are kept
/// as-is (they cannot be resolved without a base).
fn normalize_path(path: PathBuf) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                let last_is_normal =
                    matches!(out.components().next_back(), Some(Component::Normal(_)));
                if last_is_normal {
                    out.pop();
                } else if !out.has_root() {
                    // Keep leading `..` on relative paths.
                    out.push(component.as_os_str());
                }
                // For absolute paths at root, `..` is a no-op (silently dropped).
            }
        }
    }
    out
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

    // ── RgbaColor / FreezeColors tests live in src/config/colors.rs ─────────

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

    // ── normalize_path ────────────────────────────────────────────────────────

    #[test]
    fn test_normalize_path_dotdot_collapses_normal() {
        assert_eq!(
            normalize_path(PathBuf::from("/home/user/foo/../bar")),
            PathBuf::from("/home/user/bar"),
        );
    }

    #[test]
    fn test_normalize_path_root_dotdot_stays_at_root() {
        // `/../etc` must not pop the root component – result is `/etc`.
        assert_eq!(
            normalize_path(PathBuf::from("/../etc")),
            PathBuf::from("/etc"),
        );
    }

    #[test]
    fn test_normalize_path_relative_leading_dotdot_preserved() {
        // Relative leading `..` cannot be resolved without a base, keep them.
        assert_eq!(
            normalize_path(PathBuf::from("../outside")),
            PathBuf::from("../outside"),
        );
    }

    #[test]
    fn test_expand_tilde_traversal_clamped_to_screenshots() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        // `~/foo/../../etc` normalises to `/<home_parent>/etc` which is
        // outside home, so expand_tilde must fall back to ~/Screenshots.
        let result = expand_tilde(&PathBuf::from("~/foo/../../etc"));
        assert_eq!(result, home.join("Screenshots"));
    }

    #[test]
    fn test_expand_tilde_absolute_path_is_passed_through() {
        // An explicit absolute path is never subject to the home-escape guard.
        assert_eq!(
            expand_tilde(&PathBuf::from("/tmp/screenshots")),
            PathBuf::from("/tmp/screenshots"),
        );
    }
}
