mod app;

use app::{AppState, Message, app_subscription, app_update, app_view};
use iced::Task;
use iced::widget::image as iced_image;
use iced_layershell::{
    reexport::{Anchor, IcedId, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption},
    settings::LayerShellSettings,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};
use tempfile::NamedTempFile;

use crate::cmd::CMD_GRIM;
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::hyprland::{self, ScreenRect};

pub fn run_freeze(cfg: &Config) -> Result<PathBuf> {
    let tmp = NamedTempFile::with_suffix(".png")
        .map_err(|e| AppError::Other(format!("failed to create temp file: {}", e)))?;
    let tmp_path = tmp.path().to_owned();

    let tmp_path_str = tmp_path
        .to_str()
        .ok_or_else(|| AppError::Other("Temp path contains invalid UTF-8".to_string()))?;

    let mut grim_child = Command::new(CMD_GRIM)
        .arg(tmp_path_str)
        .spawn()
        .map_err(|e| AppError::CommandNotFound(CMD_GRIM.to_string(), e))?;

    let monitors_t = std::thread::spawn(hyprland::get_monitors);
    let clients_t = std::thread::spawn(hyprland::get_clients);

    let grim_status = grim_child
        .wait()
        .map_err(|e| AppError::Other(format!("grim wait failed: {}", e)))?;

    if !grim_status.success() {
        return Err(AppError::CommandFailed(CMD_GRIM.to_string(), grim_status));
    }

    let monitors_raw = monitors_t.join().expect("monitors thread panicked")?;
    let clients_raw = clients_t.join().expect("clients thread panicked")?;

    let monitors = hyprland::parse_monitors(monitors_raw);
    let active_ws_ids: Vec<i64> = monitors.iter().map(|m| m.active_workspace_id).collect();
    let windows = hyprland::parse_windows(clients_raw, &active_ws_ids);

    let full_rgba = ::image::open(&tmp_path)?.into_rgba8();

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

    let focused_monitor_idx = monitors.iter().position(|m| m.focused).unwrap_or(0);

    let (fw, fh) = {
        let m = &monitors[focused_monitor_idx];
        (m.rect.w as u32, m.rect.h as u32)
    };

    let result: Arc<Mutex<Option<Option<ScreenRect>>>> = Arc::new(Mutex::new(None));

    {
        let result_clone = result.clone();
        let mut window_to_monitor: HashMap<IcedId, usize> = HashMap::new();

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
            size: Some((fw, fh)),
            ..Default::default()
        })
        .run()
        .map_err(|e| AppError::LayerShell(e.to_string()))?;
    }

    let selected = result.lock().unwrap().take();

    match selected {
        None => Err(AppError::UserCancelled),
        Some(region) => {
            let out_path = cfg.output_path();
            crop_and_save(full_rgba, region, &out_path)?;
            Ok(out_path)
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

    cropped.save(dst).map_err(AppError::from)
}
