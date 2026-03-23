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
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};
use tempfile::NamedTempFile;

use crate::config::Config;

/// Full freeze-mode flow:
/// 1. Capture all monitors to a single composite PNG via grim
/// 2. Pre-decode the image and crop one handle per monitor (avoids async loading lag)
/// 3. Launch iced_layershell daemon — initial window on focused monitor,
///    extra windows spawned at boot for every other monitor
/// 4. Each window renders only its monitor's slice of the screenshot
/// 5. Crop the composite PNG to the selected region and save
///
/// Returns the saved path, or `None` if the user cancelled.
pub fn run_freeze(cfg: &Config) -> Result<Option<PathBuf>> {
    // ── Step 1: capture + metadata (parallel) ────────────────────────────────
    let tmp = NamedTempFile::with_suffix(".png").context("failed to create temp file")?;
    let tmp_path = tmp.path().to_owned();

    // Spawn grim without waiting so IPC queries can run concurrently.
    let mut grim_child = Command::new("grim")
        .arg(tmp_path.to_str().unwrap())
        .spawn()
        .context("failed to spawn grim")?;

    // Fetch raw monitor + client JSON via Hyprland IPC socket in parallel.
    let monitors_t = std::thread::spawn(overlay::fetch_monitors_raw);
    let clients_t = std::thread::spawn(overlay::fetch_clients_raw);

    let grim_status = grim_child.wait().context("grim wait failed")?;
    if !grim_status.success() {
        anyhow::bail!("grim failed to capture screen");
    }

    let monitors_raw = monitors_t
        .join()
        .expect("monitors thread panicked")
        .unwrap_or_default();
    let clients_raw = clients_t
        .join()
        .expect("clients thread panicked")
        .unwrap_or_default();

    // ── Step 2: decode image + build per-monitor handles ─────────────────────
    let monitors = overlay::parse_monitors(monitors_raw);
    let active_ws_ids: Vec<i64> = monitors.iter().map(|m| m.active_workspace_id).collect();
    let windows = overlay::parse_windows(clients_raw, &active_ws_ids);

    // Decode once and convert to RGBA8 upfront.
    // grim outputs RGB PNG; converting the full image here avoids a per-monitor
    // channel conversion inside the crop loop.
    let full_rgba = ::image::open(&tmp_path)
        .context("failed to decode screenshot")?
        .into_rgba8();

    // Build a per-monitor image handle pre-decoded as RGBA bytes so iced
    // renders it immediately (no async loading stall on first frame)
    let monitor_images: Vec<iced_image::Handle> = monitors
        .iter()
        .map(|m| {
            let x = m.rect.x.max(0) as u32;
            let y = m.rect.y.max(0) as u32;
            let w = (m.rect.w as u32).min(full_rgba.width().saturating_sub(x));
            let h = (m.rect.h as u32).min(full_rgba.height().saturating_sub(y));
            let cropped = ::image::imageops::crop_imm(&full_rgba, x, y, w, h).to_image();
            iced_image::Handle::from_rgba(cropped.width(), cropped.height(), cropped.into_raw())
        })
        .collect();

    // Index of the focused monitor (fallback: 0)
    let focused_monitor_idx = monitors.iter().position(|m| m.focused).unwrap_or(0);

    // Focused monitor size for daemon's required `size` field
    let (fw, fh) = {
        let m = &monitors[focused_monitor_idx];
        (m.rect.w as u32, m.rect.h as u32)
    };

    // ── Step 3: launch overlay UI ─────────────────────────────────────────────
    let result: Arc<Mutex<Option<Option<ScreenRect>>>> = Arc::new(Mutex::new(None));

    {
        let result_clone = result.clone();

        // Build window_to_monitor map for non-focused monitors.
        // We create IcedIds here so we can tell app_view which monitor each window is on.
        let mut window_to_monitor: HashMap<IcedId, usize> = HashMap::new();

        // Pre-build (IcedId, settings) pairs — Task can't be cloned but these can
        let extra_specs: Vec<(IcedId, NewLayerShellSettings)> = monitors
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.focused)
            .map(|(idx, m)| {
                let id = IcedId::unique();
                window_to_monitor.insert(id, idx);
                let settings = NewLayerShellSettings {
                    size: Some((m.rect.w as u32, m.rect.h as u32)),
                    layer: Layer::Overlay,
                    anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: OutputOption::OutputName(m.name.clone()),
                    namespace: Some("crop-hypr-freeze".to_string()),
                    ..Default::default()
                };
                (id, settings)
            })
            .collect();

        let extra_specs = std::sync::Arc::new(extra_specs);

        let wins = Arc::new(windows);
        let mons = Arc::new(monitors);

        iced_layershell::daemon(
            move || {
                let spawn_tasks: Vec<Task<Message>> = extra_specs
                    .iter()
                    .map(|(id, settings)| {
                        Task::done(Message::NewLayerShell {
                            settings: settings.clone(),
                            id: *id,
                        })
                    })
                    .collect();

                let state = AppState::new(
                    monitor_images.clone(),
                    focused_monitor_idx,
                    window_to_monitor.clone(),
                    Arc::clone(&wins),
                    Arc::clone(&mons),
                    result_clone.clone(),
                );
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
            // daemon() requires an explicit size; compositor overrides it to the
            // actual monitor dimensions because we anchor to all four edges
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
            crop_and_save(full_rgba, region, &out_path)?;
            Ok(Some(out_path))
        }
    }
}

fn crop_and_save(
    img: ::image::ImageBuffer<::image::Rgba<u8>, Vec<u8>>,
    region: Option<ScreenRect>,
    dst: &std::path::Path,
) -> Result<()> {
    let cropped = match region {
        None => ::image::DynamicImage::ImageRgba8(img),
        Some(r) => {
            let x = r.x.max(0) as u32;
            let y = r.y.max(0) as u32;
            let w = (r.w as u32).min(img.width().saturating_sub(x));
            let h = (r.h as u32).min(img.height().saturating_sub(y));
            ::image::DynamicImage::ImageRgba8(
                ::image::imageops::crop_imm(&img, x, y, w, h).to_image(),
            )
        }
    };

    cropped.save(dst).context("failed to save cropped image")
}
