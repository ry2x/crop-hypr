mod app;

pub use app::CaptureMode;

use app::{AppState, AppStateConfig, Message, app_subscription, app_update, app_view};
use iced::Task;
use iced::widget::image as iced_image;
use iced_layershell::{
    reexport::{Anchor, IcedId, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption},
    settings::LayerShellSettings,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::freeze_state;
use crate::hyprland::{self, ScreenRect};
use crate::screencopy;

pub fn run_freeze(cfg: &Config) -> Result<PathBuf> {
    let monitors_t = std::thread::spawn(hyprland::get_monitors);
    let clients_t = std::thread::spawn(hyprland::get_clients);
    let layers_t = std::thread::spawn(hyprland::get_overlay_layers);
    let border_style = if cfg.capture_window_border {
        hyprland::get_border_style()
    } else {
        hyprland::BorderStyle::default()
    };
    let initial_mode = freeze_state::load_last_mode();

    let monitors_raw = monitors_t.join().expect("monitors thread panicked")?;
    let clients_raw = clients_t.join().expect("clients thread panicked")?;
    let layers = layers_t
        .join()
        .expect("layers thread panicked")
        .unwrap_or_default();

    let monitors = hyprland::parse_monitors(monitors_raw);
    let active_ws_ids: Vec<i64> = monitors.iter().map(|m| m.active_workspace_id).collect();
    let windows = hyprland::parse_windows(clients_raw, &active_ws_ids);

    // Compute origin before monitors are moved into Arc.
    // capture_all_monitors places (min_x, min_y) at image pixel (0,0); we need this
    // to translate the UI's global logical coordinates into image coordinates later.
    let min_x = monitors.iter().map(|m| m.rect.x).min().unwrap_or(0);
    let min_y = monitors.iter().map(|m| m.rect.y).min().unwrap_or(0);

    // Capture all monitors in a single Wayland session.
    // Using one session guarantees the overlay images and the final-crop source are
    // from the same frame — two separate captures would differ in time, breaking
    // the "freeze" guarantee (user selects based on a different frame than what gets saved).
    let (physical_per_monitor, full_rgba) =
        screencopy::capture_all_monitors_with_physical(&monitors)?;

    let monitor_images: Vec<iced_image::Handle> = physical_per_monitor
        .into_iter()
        .map(|img| iced_image::Handle::from_rgba(img.width(), img.height(), img.into_raw()))
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
        let lyrs = Arc::new(layers);
        let glyphs = cfg.freeze_glyphs.clone();
        let toolbar_position = cfg.toolbar_position;
        let colors = cfg.freeze_colors;

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

                let state = AppState::new(AppStateConfig {
                    monitor_images: monitor_images.clone(),
                    focused_monitor_idx,
                    window_to_monitor: window_to_monitor.clone(),
                    windows: Arc::clone(&wins),
                    monitors: Arc::clone(&mons),
                    layers: Arc::clone(&lyrs),
                    result: result_clone.clone(),
                    glyphs: glyphs.clone(),
                    toolbar_position,
                    border_style,
                    initial_mode,
                    colors,
                });
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

    let selected = result
        .lock()
        .expect("UI thread panicked and poisoned result mutex")
        .take();

    match selected {
        None => Err(AppError::UserCancelled),
        Some(region) => {
            let out_path = cfg.output_path();
            // Translate global logical coordinates → image coordinates.
            // The image origin is (min_x, min_y) in global logical space.
            let adjusted = region.map(|r| ScreenRect {
                x: r.x - min_x,
                y: r.y - min_y,
                w: r.w,
                h: r.h,
            });
            crop_and_save(full_rgba, adjusted, &out_path)?;
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
