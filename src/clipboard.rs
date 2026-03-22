use anyhow::{Context, Result};
use std::{path::Path, process::Command};

/// Copy the image at `path` to the Wayland clipboard via `wl-copy`.
pub fn copy_to_clipboard(path: &Path) -> Result<()> {
    let status = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(std::fs::File::open(path).with_context(|| {
            format!(
                "failed to open screenshot for clipboard: {}",
                path.display()
            )
        })?)
        .status()
        .context("failed to spawn wl-copy — is wl-clipboard installed?")?;

    if !status.success() {
        anyhow::bail!("wl-copy exited with non-zero status");
    }

    Ok(())
}
