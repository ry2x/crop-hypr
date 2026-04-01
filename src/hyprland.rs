use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

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

/// Resolve Hyprland's IPC socket path from environment variables.
fn hyprland_socket_path() -> Result<PathBuf> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .context("HYPRLAND_INSTANCE_SIGNATURE not set – is Hyprland running?")?;
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime)
            .join("hypr")
            .join(&sig)
            .join(".socket.sock");
        if p.exists() {
            return Ok(p);
        }
    }
    Ok(PathBuf::from(format!("/tmp/hypr/{}/.socket.sock", sig)))
}

/// Send a single request to Hyprland's IPC socket and return the raw JSON response.
pub fn hyprland_ipc(cmd: &str) -> Result<serde_json::Value> {
    let path = hyprland_socket_path()?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("failed to connect to Hyprland socket at {}", path.display()))?;
    write!(stream, "j/{}", cmd).context("failed to write to Hyprland socket")?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .context("failed to shutdown socket write half")?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .context("failed to read from Hyprland socket")?;
    serde_json::from_slice(&buf).context("failed to parse Hyprland IPC response as JSON")
}

pub fn parse_monitors(monitors: serde_json::Value) -> Vec<MonitorInfo> {
    let arr = match monitors.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
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
        .collect()
}

pub fn parse_windows(clients: serde_json::Value, active_workspace_ids: &[i64]) -> Vec<WindowInfo> {
    let arr = match clients.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|c| {
            if c["hidden"].as_bool().unwrap_or(false) {
                return None;
            }
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
        .collect()
}
