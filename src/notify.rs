use anyhow::{Context, Result};
use std::{path::Path, process::Command};

pub fn notify_success(path: &Path) {
    let path_str = path.display().to_string();
    let _ = Command::new("notify-send")
        .args([
            "--app-name=crop-hypr",
            &format!("--icon={path_str}"),
            "Screenshot saved",
            &path_str,
        ])
        .status();
}

pub fn notify_error(msg: &str) -> Result<()> {
    Command::new("notify-send")
        .args([
            "--app-name=crop-hypr",
            "--icon=dialog-error",
            "--urgency=critical",
            "Screenshot failed",
            msg,
        ])
        .status()
        .context("failed to spawn notify-send")?;
    Ok(())
}
