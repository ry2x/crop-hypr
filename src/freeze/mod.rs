mod app;
mod overlay;

use anyhow::{Context, Result};
use app::{AppState, Message, app_subscription, app_update, app_view};
use iced::Task;
use iced::widget::image as iced_image;
use iced_layershell::{
    reexport::{Anchor, IcedId, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption},
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
/// 3. Launch the iced_layershell overlay UI — opens on the focused monitor and
///    spawns additional windows for every other connected monitor at boot
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

    // Focused monitor size for surface initialisation (daemon requires explicit size)
    let (fw, fh) = monitors
        .iter()
        .find(|m| m.focused)
        .map(|m| (m.rect.w as u32, m.rect.h as u32))
        .unwrap_or((1920, 1080));

    // Non-focused monitors need their own overlay windows spawned at boot
    let extra_monitors: Vec<_> = monitors.iter().filter(|m| !m.focused).cloned().collect();

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

        iced_layershell::daemon(
            move || {
                let state = AppState::new(
                    img_handle.clone(),
                    wins.clone(),
                    mons.clone(),
                    result_clone.clone(),
                );
                // Spawn an overlay window on every non-focused monitor
                let spawn_tasks: Vec<Task<Message>> = extra_monitors
                    .iter()
                    .map(|m| {
                        let id = IcedId::unique();
                        let settings = NewLayerShellSettings {
                            layer: Layer::Overlay,
                            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                            exclusive_zone: Some(-1),
                            keyboard_interactivity: KeyboardInteractivity::Exclusive,
                            output_option: OutputOption::OutputName(m.name.clone()),
                            namespace: Some("crop-hypr-freeze".to_string()),
                            ..Default::default()
                        };
                        Task::done(Message::NewLayerShell { settings, id })
                    })
                    .collect();
                (state, Task::batch(spawn_tasks))
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
            // daemon requires explicit size; compositor overrides via configure
            // event because we anchor to all four edges
            size: Some((fw, fh)),
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
