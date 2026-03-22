use anyhow::{Context, Result, bail};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::config::Config;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Run `slurp` and return the selected geometry string (e.g. "100,200 640x480").
/// Returns `None` if the user cancelled (slurp exits with code 1).
fn slurp_region() -> Result<Option<String>> {
    let output = Command::new("slurp")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("failed to spawn slurp — is it installed?")?;

    if !output.status.success() {
        return Ok(None); // cancelled
    }

    let region = String::from_utf8(output.stdout)
        .context("slurp output is not valid UTF-8")?
        .trim()
        .to_owned();

    if region.is_empty() {
        bail!("slurp returned empty geometry");
    }

    Ok(Some(region))
}

/// Build the output path from config, creating the directory if needed.
fn make_output_path(cfg: &Config) -> Result<PathBuf> {
    std::fs::create_dir_all(&cfg.save_path)
        .with_context(|| format!("failed to create directory: {}", cfg.save_path.display()))?;
    Ok(cfg.output_path())
}

/// Invoke `grim` with the given extra args and save to `path`.
fn run_grim(extra_args: &[&str], path: &std::path::Path) -> Result<()> {
    let path_str = path.to_str().unwrap();
    let status = Command::new("grim")
        .args(extra_args)
        .arg(path_str)
        .status()
        .context("failed to spawn grim — is it installed?")?;

    if !status.success() {
        bail!("grim exited with non-zero status");
    }
    Ok(())
}

/// Run `hyprctl -j <cmd>` and parse the JSON output.
fn hyprctl_json(cmd: &str) -> Result<serde_json::Value> {
    let output = Command::new("hyprctl")
        .args(["-j", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("failed to spawn hyprctl — is Hyprland running?")?;

    if !output.status.success() {
        bail!("hyprctl {cmd} failed");
    }

    serde_json::from_slice(&output.stdout).context("failed to parse hyprctl JSON")
}

// ── public capture functions ──────────────────────────────────────────────────

/// Capture a user-selected crop region via slurp.
/// Returns `None` if the user cancelled.
pub fn capture_crop(cfg: &Config) -> Result<Option<PathBuf>> {
    let Some(region) = slurp_region()? else {
        return Ok(None);
    };

    let path = make_output_path(cfg)?;
    run_grim(&["-g", &region], &path)?;
    Ok(Some(path))
}

/// Capture the currently active window using its geometry from hyprctl.
pub fn capture_window(cfg: &Config) -> Result<PathBuf> {
    let info = hyprctl_json("activewindow")?;

    let x = info["at"][0]
        .as_i64()
        .context("activewindow: missing at[0]")?;
    let y = info["at"][1]
        .as_i64()
        .context("activewindow: missing at[1]")?;
    let w = info["size"][0]
        .as_i64()
        .context("activewindow: missing size[0]")?;
    let h = info["size"][1]
        .as_i64()
        .context("activewindow: missing size[1]")?;

    let region = format!("{x},{y} {w}x{h}");
    let path = make_output_path(cfg)?;
    run_grim(&["-g", &region], &path)?;
    Ok(path)
}

/// Capture the focused monitor by output name.
pub fn capture_monitor(cfg: &Config) -> Result<PathBuf> {
    let monitors = hyprctl_json("monitors")?;

    let focused = monitors
        .as_array()
        .context("monitors: expected JSON array")?
        .iter()
        .find(|m| m["focused"].as_bool().unwrap_or(false))
        .context("no focused monitor found")?;

    let name = focused["name"]
        .as_str()
        .context("monitor: missing name field")?;

    let path = make_output_path(cfg)?;
    run_grim(&["-o", name], &path)?;
    Ok(path)
}

/// Capture all monitors (grim default — no geometry flag).
pub fn capture_all(cfg: &Config) -> Result<PathBuf> {
    let path = make_output_path(cfg)?;
    run_grim(&[], &path)?;
    Ok(path)
}
