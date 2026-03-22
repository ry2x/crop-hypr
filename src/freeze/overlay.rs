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

pub fn fetch_windows() -> Result<Vec<WindowInfo>> {
    let clients = hyprctl_json("clients")?;
    let arr = clients.as_array().context("clients: expected array")?;

    Ok(arr
        .iter()
        .filter_map(|c| {
            let x = c["at"][0].as_i64()? as i32;
            let y = c["at"][1].as_i64()? as i32;
            let w = c["size"][0].as_i64()? as i32;
            let h = c["size"][1].as_i64()? as i32;
            let title = c["title"].as_str().unwrap_or("").to_owned();
            // skip windows with no size
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
            Some(MonitorInfo {
                rect: ScreenRect { x, y, w, h },
                name,
                focused,
            })
        })
        .collect())
}
