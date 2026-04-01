use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowCaptureMethod {
    #[default]
    Geometry,
    // TODO: implement xdg-desktop-portal (ashpd) based window capture
    Portal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Directory where screenshots are saved.
    #[serde(default = "default_save_path")]
    pub save_path: PathBuf,

    /// How to capture individual windows.
    #[serde(default)]
    pub window_capture_method: WindowCaptureMethod,

    /// strftime-style filename template (no extension).
    #[serde(default = "default_filename_pattern")]
    pub filename_pattern: String,
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
            window_capture_method: WindowCaptureMethod::default(),
            filename_pattern: default_filename_pattern(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;

        let mut cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;

        cfg.save_path = expand_tilde(&cfg.save_path);
        Ok(cfg)
    }

    /// Expand `filename_pattern` with current timestamp and append `.png`.
    pub fn output_filename(&self) -> String {
        let ts = chrono::Local::now()
            .format(&self.filename_pattern)
            .to_string();
        format!("{ts}.png")
    }

    /// Full path for the next screenshot.
    pub fn output_path(&self) -> PathBuf {
        self.save_path.join(self.output_filename())
    }
}

fn config_path() -> PathBuf {
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
