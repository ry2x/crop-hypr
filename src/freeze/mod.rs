mod app;
mod overlay;

use anyhow::{Context, Result};
use app::{AppState, app_subscription, app_update, app_view};
use iced::widget::image as iced_image;
use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::LayerShellSettings,
};
use overlay::ScreenRect;
use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};
use tempfile::NamedTempFile;

use crate::config::Config;

/// Full freeze-mode flow:
/// 1. Capture all monitors to a temp PNG
/// 2. Load geometry info from hyprctl
/// 3. Launch the iced_layershell overlay UI
/// 4. Read the selected region from the shared mutex
/// 5. Crop the temp PNG and save to the output path
///
/// Returns the saved path, or `None` if the user cancelled.
pub fn run_freeze(cfg: &Config) -> Result<Option<PathBuf>> {
    // ── Step 1: capture full screen ──────────────────────────────────────────
    let tmp = NamedTempFile::with_suffix(".png").context("failed to create temp file")?;
    let tmp_path = tmp.path().to_owned();

    let status = Command::new("grim")
        .arg(tmp_path.to_str().unwrap())
        .status()
        .context("failed to spawn grim")?;
    if !status.success() {
        anyhow::bail!("grim failed to capture screen");
    }

    // ── Step 2: load geometry from hyprctl ───────────────────────────────────
    let windows = overlay::fetch_windows().unwrap_or_default();
    let monitors = overlay::fetch_monitors().unwrap_or_default();

    // ── Step 3: launch overlay UI ─────────────────────────────────────────────
    // None        = cancelled (ESC)
    // Some(None)  = "All" selected → use full screenshot
    // Some(Some)  = specific region
    let result: Arc<Mutex<Option<Option<ScreenRect>>>> = Arc::new(Mutex::new(None));

    {
        let result_clone = result.clone();
        let img_handle = iced_image::Handle::from_path(&tmp_path);
        let wins = windows.clone();
        let mons = monitors.clone();

        iced_layershell::application(
            move || {
                AppState::new(
                    img_handle.clone(),
                    wins.clone(),
                    mons.clone(),
                    result_clone.clone(),
                )
            },
            "crop-hypr-freeze",
            app_update,
            app_view,
        )
        .subscription(app_subscription)
        .layer_settings(LayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        })
        .run()
        .context("overlay UI failed")?;
    }

    // ── Step 4: read result ───────────────────────────────────────────────────
    let selected = result.lock().unwrap().take();

    match selected {
        None => Ok(None), // cancelled
        Some(region) => {
            std::fs::create_dir_all(&cfg.save_path)?;
            let out_path = cfg.output_path();
            crop_and_save(&tmp_path, region, &out_path)?;
            Ok(Some(out_path))
        }
    }
}

fn crop_and_save(
    src: &std::path::Path,
    region: Option<ScreenRect>,
    dst: &std::path::Path,
) -> Result<()> {
    let img = ::image::open(src).context("failed to open temp screenshot")?;

    let cropped = match region {
        None => img,
        Some(r) => {
            let x = r.x.max(0) as u32;
            let y = r.y.max(0) as u32;
            let w = (r.w as u32).min(img.width().saturating_sub(x));
            let h = (r.h as u32).min(img.height().saturating_sub(y));
            img.crop_imm(x, y, w, h)
        }
    };

    cropped.save(dst).context("failed to save cropped image")
}
