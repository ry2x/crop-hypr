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
pub struct HyprClient {
    pub hidden: bool,
    pub workspace: HyprWorkspace,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub title: String,
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
    let parsed: T = serde_json::from_slice(&buf)?;
    Ok(parsed)
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

pub fn parse_windows(clients: Vec<HyprClient>, active_workspace_ids: &[i64]) -> Vec<WindowInfo> {
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
            })
        })
        .collect()
}
