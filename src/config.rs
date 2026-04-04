use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

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

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_save_path")]
    pub save_path: PathBuf,

    #[serde(default = "default_filename_pattern")]
    pub filename_pattern: String,

    #[serde(default)]
    pub freeze_glyphs: FreezeGlyphs,
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
    pub fn generate_default_toml() -> String {
        toml::to_string_pretty(&Self::default()).expect("default Config must be serializable")
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

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert!(cfg.save_path.to_string_lossy().contains("Screenshots"));
        assert_eq!(cfg.filename_pattern, "hyprsnap_%Y%m%d_%H%M%S");
        assert_eq!(cfg.freeze_glyphs.crop, "\u{F019F}");
        assert_eq!(cfg.freeze_glyphs.window, "\u{EB7F}");
        assert_eq!(cfg.freeze_glyphs.monitor, "\u{F0379}");
        assert_eq!(cfg.freeze_glyphs.all, "\u{F004C}");
        assert_eq!(cfg.freeze_glyphs.cancel, "\u{F05AD}");
    }

    #[test]
    fn test_config_validation() {
        let cfg: Config = toml::from_str("filename_pattern = 'test'").unwrap();
        assert!(cfg.validate().is_ok());

        let cfg: Config = toml::from_str("filename_pattern = ''").unwrap();
        let res = cfg.validate();
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("filename_pattern cannot be empty")
        );

        let cfg2: Config = toml::from_str("filename_pattern = '   '").unwrap();
        assert!(cfg2.validate().is_err());
    }

    #[test]
    fn test_freeze_glyphs_partial_override() {
        let cfg: Config = toml::from_str("[freeze_glyphs]\ncrop = \"X\"").unwrap();
        assert_eq!(cfg.freeze_glyphs.crop, "X");
        // Unspecified fields fall back to defaults.
        assert_eq!(cfg.freeze_glyphs.cancel, default_glyph_cancel());
    }

    #[test]
    fn test_generate_default_toml() {
        let toml_str = Config::generate_default_toml();
        // Must be valid TOML and round-trip cleanly.
        let parsed: Config = toml::from_str(&toml_str).expect("generated TOML must be parseable");
        assert_eq!(parsed.filename_pattern, default_filename_pattern());
        assert_eq!(parsed.freeze_glyphs.crop, default_glyph_crop());
    }

    #[test]
    fn test_tilde_expansion() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        let path = PathBuf::from("~/test/dir");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home.join("test/dir"));

        let path = PathBuf::from("~");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home);

        let path = PathBuf::from("/tmp/test");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, PathBuf::from("/tmp/test"));

        let path = PathBuf::from("Screenshots");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home.join("Screenshots"));
    }
}
