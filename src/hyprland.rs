use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl ScreenRect {
    /// Expand the rect outward by `border_size` on every side (in logical pixels).
    pub fn expand(self, border_size: u32) -> Self {
        let b = border_size as i32;
        Self {
            x: self.x - b,
            y: self.y - b,
            w: self.w + 2 * b,
            h: self.h + 2 * b,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub rect: ScreenRect,
    pub title: String,
    pub floating: bool,
    /// Lower = more recently focused (0 = topmost floating window).
    pub focus_history_id: i64,
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub rect: ScreenRect,
    pub name: String,
    pub focused: bool,
    pub active_workspace_id: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HyprMonitor {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub name: String,
    pub focused: bool,
    pub active_workspace: HyprWorkspace,
}

#[derive(Deserialize, Debug)]
pub struct HyprWorkspace {
    pub id: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HyprClient {
    pub hidden: bool,
    pub workspace: HyprWorkspace,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub title: String,
    pub floating: bool,
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct HyprActiveWindow {
    pub at: [i32; 2],
    pub size: [i32; 2],
}

fn hyprland_socket_path() -> Result<PathBuf> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map_err(|e| AppError::HyprlandEnvVar("HYPRLAND_INSTANCE_SIGNATURE", e))?;
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

pub fn hyprland_ipc_raw(cmd: &str) -> Result<Vec<u8>> {
    let path = hyprland_socket_path()?;
    let mut stream =
        UnixStream::connect(&path).map_err(|e| AppError::HyprlandIpc(cmd.to_string(), e))?;
    write!(stream, "j/{}", cmd).map_err(|e| AppError::HyprlandIpc(cmd.to_string(), e))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| AppError::HyprlandIpc(cmd.to_string(), e))?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| AppError::HyprlandIpc(cmd.to_string(), e))?;
    Ok(buf)
}

pub fn hyprland_ipc<T: for<'de> Deserialize<'de>>(cmd: &str) -> Result<T> {
    let buf = hyprland_ipc_raw(cmd)?;
    let parsed: T =
        serde_json::from_slice(&buf).map_err(|e| AppError::HyprlandJson(cmd.to_string(), e))?;
    Ok(parsed)
}

#[derive(Deserialize, Debug)]
pub struct HyprOption {
    pub int: i64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BorderStyle {
    /// Hyprland `general:border_size` in logical pixels.
    pub border_size: u32,
    /// Hyprland `decoration:rounding` in logical pixels.
    pub rounding: u32,
}

/// Fetch `general:border_size` and `decoration:rounding` from Hyprland IPC.
/// Falls back to `BorderStyle::default()` on any error.
pub fn get_border_style() -> BorderStyle {
    let bs = hyprland_ipc::<HyprOption>("getoption general:border_size")
        .map(|o| o.int.max(0) as u32)
        .unwrap_or(0);
    let rd = hyprland_ipc::<HyprOption>("getoption decoration:rounding")
        .map(|o| o.int.max(0) as u32)
        .unwrap_or(0);
    BorderStyle {
        border_size: bs,
        rounding: rd,
    }
}

pub fn get_active_window() -> Result<HyprActiveWindow> {
    hyprland_ipc("activewindow")
}

pub fn get_monitors() -> Result<Vec<HyprMonitor>> {
    hyprland_ipc("monitors")
}

pub fn get_clients() -> Result<Vec<HyprClient>> {
    hyprland_ipc("clients")
}

pub fn parse_monitors(monitors: Vec<HyprMonitor>) -> Vec<MonitorInfo> {
    monitors
        .into_iter()
        .map(|m| MonitorInfo {
            rect: ScreenRect {
                x: m.x,
                y: m.y,
                w: m.width,
                h: m.height,
            },
            name: m.name,
            focused: m.focused,
            active_workspace_id: m.active_workspace.id,
        })
        .collect()
}

pub(crate) fn parse_windows(
    clients: Vec<HyprClient>,
    active_workspace_ids: &[i64],
) -> Vec<WindowInfo> {
    clients
        .into_iter()
        .filter_map(|c| {
            if c.hidden {
                return None;
            }
            if !active_workspace_ids.contains(&c.workspace.id) {
                return None;
            }
            let w = c.size[0];
            let h = c.size[1];
            if w <= 0 || h <= 0 {
                return None;
            }
            Some(WindowInfo {
                rect: ScreenRect {
                    x: c.at[0],
                    y: c.at[1],
                    w,
                    h,
                },
                title: c.title,
                floating: c.floating,
                focus_history_id: c.focus_history_id,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_parsing() {
        let monitors = vec![HyprMonitor {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            name: "DP-1".to_string(),
            focused: true,
            active_workspace: HyprWorkspace { id: 1 },
        }];

        let parsed = parse_monitors(monitors);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "DP-1");
        assert_eq!(parsed[0].rect.w, 1920);
        assert!(parsed[0].focused);
        assert_eq!(parsed[0].active_workspace_id, 1);
    }

    #[test]
    fn test_window_parsing() {
        let clients = vec![
            HyprClient {
                hidden: false,
                workspace: HyprWorkspace { id: 1 },
                at: [100, 100],
                size: [800, 600],
                title: "Visible Window".to_string(),
                floating: false,
                focus_history_id: 1,
            },
            HyprClient {
                hidden: true,
                workspace: HyprWorkspace { id: 1 },
                at: [200, 200],
                size: [800, 600],
                title: "Hidden Window".to_string(),
                floating: false,
                focus_history_id: 2,
            },
            HyprClient {
                hidden: false,
                workspace: HyprWorkspace { id: 2 },
                at: [300, 300],
                size: [800, 600],
                title: "Other Workspace Window".to_string(),
                floating: false,
                focus_history_id: 3,
            },
        ];

        let active_workspaces = vec![1];
        let parsed = parse_windows(clients, &active_workspaces);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, "Visible Window");
        assert_eq!(parsed[0].rect.x, 100);
        assert_eq!(parsed[0].rect.w, 800);
    }
}
