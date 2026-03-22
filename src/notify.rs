use anyhow::{Context, Result};
use std::{path::Path, process::Command};

pub fn notify_success(path: &Path) {
    let _ = Command::new("notify-send")
        .args([
            "--app-name=crop-hypr",
            "--icon=camera-photo",
            "Screenshot saved",
            &path.display().to_string(),
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
