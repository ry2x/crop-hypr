use anyhow::{Context, Result, bail};
use chrono::Local;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

/// Run `slurp` and return the selected geometry string (e.g. "100,200 640x480").
/// Returns `None` if the user cancelled (slurp exits with code 1).
fn slurp_region() -> Result<Option<String>> {
    let output = Command::new("slurp")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("failed to spawn slurp — is it installed?")?;

    if !output.status.success() {
        return Ok(None); // cancelled
    }

    let region = String::from_utf8(output.stdout)
        .context("slurp output is not valid UTF-8")?
        .trim()
        .to_owned();

    if region.is_empty() {
        bail!("slurp returned empty geometry");
    }

    Ok(Some(region))
}

/// Build a timestamped output path under `~/Pictures/Screenshots/`.
fn output_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    let dir = PathBuf::from(home).join("Pictures").join("Screenshots");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create directory: {}", dir.display()))?;

    let filename = Local::now()
        .format("hyprsnap_%Y%m%d_%H%M%S.png")
        .to_string();
    Ok(dir.join(filename))
}

/// Capture the selected region with `grim`.
/// Returns the path to the saved file, or `None` if the user cancelled selection.
pub fn capture_crop() -> Result<Option<PathBuf>> {
    let Some(region) = slurp_region()? else {
        return Ok(None);
    };

    let path = output_path()?;

    let status = Command::new("grim")
        .args(["-g", &region, path.to_str().unwrap()])
        .status()
        .context("failed to spawn grim — is it installed?")?;

    if !status.success() {
        bail!("grim exited with non-zero status");
    }

    Ok(Some(path))
}
