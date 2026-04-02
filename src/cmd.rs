use std::ffi::OsStr;
use std::process::{Command, Stdio};

use crate::error::{AppError, Result};

pub const CMD_SLURP: &str = "slurp";
pub const CMD_WL_COPY: &str = "wl-copy";
pub const CMD_NOTIFY_SEND: &str = "notify-send";

pub fn run_cmd_status<I, S>(cmd: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| AppError::CommandNotFound(cmd.to_string(), e))?;

    if !status.success() {
        return Err(AppError::CommandFailed(cmd.to_string(), status));
    }
    Ok(())
}

pub fn run_cmd_status_with_stdin<I, S>(cmd: &str, args: I, file: std::fs::File) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(cmd)
        .args(args)
        .stdin(Stdio::from(file))
        .status()
        .map_err(|e| AppError::CommandNotFound(cmd.to_string(), e))?;

    if !status.success() {
        return Err(AppError::CommandFailed(cmd.to_string(), status));
    }
    Ok(())
}
