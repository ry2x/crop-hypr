use anyhow::{Context, Result};
use std::process::{Command, Stdio};

/// Screen-space rectangle in logical pixels (from hyprctl).
#[derive(Debug, Clone, Copy)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub rect: ScreenRect,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub rect: ScreenRect,
    pub name: String,
    pub focused: bool,
    /// ID of the workspace currently displayed on this monitor.
    pub active_workspace_id: i64,
}

fn hyprctl_json(cmd: &str) -> Result<serde_json::Value> {
    let out = Command::new("hyprctl")
        .args(["-j", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("failed to spawn hyprctl")?;
    serde_json::from_slice(&out.stdout).context("failed to parse hyprctl JSON")
}

/// Fetch only the windows that are currently visible on screen.
///
/// `hyprctl -j clients` returns **all** windows across all workspaces.
/// Windows on inactive workspaces (including fullscreen ones) share the same
/// global coordinate space as visible windows, so without filtering they
/// appear as huge highlights that cover entire monitors.
/// We keep only windows whose workspace ID matches one of the monitors'
/// active workspaces and that are not flagged as hidden (scratchpad etc.).
pub fn fetch_windows(active_workspace_ids: &[i64]) -> Result<Vec<WindowInfo>> {
    let clients = hyprctl_json("clients")?;
    let arr = clients.as_array().context("clients: expected array")?;

    Ok(arr
        .iter()
        .filter_map(|c| {
            // Skip hidden windows (scratchpads and similar).
            if c["hidden"].as_bool().unwrap_or(false) {
                return None;
            }
            // Skip windows not on any currently visible workspace.
            let ws_id = c["workspace"]["id"].as_i64().unwrap_or(i64::MIN);
            if !active_workspace_ids.contains(&ws_id) {
                return None;
            }
            let x = c["at"][0].as_i64()? as i32;
            let y = c["at"][1].as_i64()? as i32;
            let w = c["size"][0].as_i64()? as i32;
            let h = c["size"][1].as_i64()? as i32;
            let title = c["title"].as_str().unwrap_or("").to_owned();
            if w <= 0 || h <= 0 {
                return None;
            }
            Some(WindowInfo {
                rect: ScreenRect { x, y, w, h },
                title,
            })
        })
        .collect())
}

pub fn fetch_monitors() -> Result<Vec<MonitorInfo>> {
    let monitors = hyprctl_json("monitors")?;
    let arr = monitors.as_array().context("monitors: expected array")?;

    Ok(arr
        .iter()
        .filter_map(|m| {
            let x = m["x"].as_i64()? as i32;
            let y = m["y"].as_i64()? as i32;
            let w = m["width"].as_i64()? as i32;
            let h = m["height"].as_i64()? as i32;
            let name = m["name"].as_str()?.to_owned();
            let focused = m["focused"].as_bool().unwrap_or(false);
            let active_workspace_id = m["activeWorkspace"]["id"].as_i64().unwrap_or(-1);
            Some(MonitorInfo {
                rect: ScreenRect { x, y, w, h },
                name,
                focused,
                active_workspace_id,
            })
        })
        .collect())
}
