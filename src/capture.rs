use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::cmd::{self, CMD_GRIM, CMD_SLURP};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::hyprland;

fn slurp_region() -> Result<String> {
    let output = Command::new(CMD_SLURP)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| AppError::CommandNotFound(CMD_SLURP.to_string(), e))?;

    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Err(AppError::UserCancelled);
        } else {
            return Err(AppError::CommandFailed(
                CMD_SLURP.to_string(),
                output.status,
            ));
        }
    }

    let region = String::from_utf8(output.stdout)
        .map_err(|_| AppError::Other("slurp output is not valid UTF-8".to_string()))?
        .trim()
        .to_owned();

    if region.is_empty() {
        return Err(AppError::EmptyGeometry);
    }

    Ok(region)
}

fn run_grim(extra_args: &[&str], path: &std::path::Path) -> Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| AppError::Other("Output path contains invalid UTF-8".to_string()))?;

    let mut args: Vec<&str> = extra_args.to_vec();
    args.push(path_str);

    cmd::run_cmd_status(CMD_GRIM, &args)
}

pub fn capture_crop(cfg: &Config) -> Result<PathBuf> {
    let region = slurp_region()?;
    let path = cfg.output_path();
    run_grim(&["-g", &region], &path)?;
    Ok(path)
}

pub fn capture_window(cfg: &Config) -> Result<PathBuf> {
    let info = hyprland::get_active_window()?;

    let x = info.at[0];
    let y = info.at[1];
    let w = info.size[0];
    let h = info.size[1];

    let region = format!("{x},{y} {w}x{h}");
    let path = cfg.output_path();
    run_grim(&["-g", &region], &path)?;
    Ok(path)
}

pub fn capture_monitor(cfg: &Config) -> Result<PathBuf> {
    let monitors = hyprland::get_monitors()?;

    let focused = monitors
        .into_iter()
        .find(|m| m.focused)
        .ok_or(AppError::NoFocusedMonitor)?;

    let path = cfg.output_path();
    run_grim(&["-o", &focused.name], &path)?;
    Ok(path)
}

pub fn capture_all(cfg: &Config) -> Result<PathBuf> {
    let path = cfg.output_path();
    run_grim(&[], &path)?;
    Ok(path)
}
