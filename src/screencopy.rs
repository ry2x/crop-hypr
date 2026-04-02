use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
    protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool},
};
use wayland_protocols::xdg::xdg_output::zv1::client::{zxdg_output_manager_v1, zxdg_output_v1};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};

use crate::error::{AppError, Result};
use image::{ImageBuffer, Rgba};
use memmap2::MmapMut;
use nix::sys::memfd;
use std::ffi::CString;
use std::os::fd::AsFd;

pub struct CaptureState {
    pub shm: Option<wl_shm::WlShm>,
    pub screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
    pub xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
    pub outputs: Vec<OutputInfo>,
    pub frames: Vec<FrameInfo>,
}

pub struct OutputInfo {
    pub output: wl_output::WlOutput,
    pub name: Option<String>,
    pub xdg_output: Option<zxdg_output_v1::ZxdgOutputV1>,
}

pub struct FrameInfo {
    pub frame: zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: Option<WEnum<wl_shm::Format>>,
    pub ready: bool,
    pub failed: bool,
    /// Populated when `failed` is true; carries diagnostic context.
    pub error_msg: Option<String>,
    pub mmap: Option<MmapMut>,
    pub buffer: Option<wl_buffer::WlBuffer>,
    pub name: String,
}

impl CaptureState {
    pub fn new() -> Self {
        Self {
            shm: None,
            screencopy_manager: None,
            xdg_output_manager: None,
            outputs: Vec::new(),
            frames: Vec::new(),
        }
    }
}

/// Convert a pixel at byte `offset` in `data` to RGBA based on the shm format.
///
/// Wayland shm format memory layout (little-endian):
/// - ARGB8888 / XRGB8888: bytes = [Blue, Green, Red, Alpha/X]
/// - ABGR8888 / XBGR8888: bytes = [Red, Green, Blue, Alpha/X]
fn read_pixel_rgba(data: &[u8], offset: usize, format: WEnum<wl_shm::Format>) -> Rgba<u8> {
    // Use `get` + `expect` to make potential out-of-bounds explicit and avoid raw indexing.
    let b0 = *data
        .get(offset)
        .expect("read_pixel_rgba: offset is out of bounds for data slice");
    let b1 = *data
        .get(offset + 1)
        .expect("read_pixel_rgba: offset + 1 is out of bounds for data slice");
    let b2 = *data
        .get(offset + 2)
        .expect("read_pixel_rgba: offset + 2 is out of bounds for data slice");
    match format {
        WEnum::Value(wl_shm::Format::Argb8888) | WEnum::Value(wl_shm::Format::Xrgb8888) => {
            Rgba([b2, b1, b0, 255])
        }
        WEnum::Value(wl_shm::Format::Abgr8888) | WEnum::Value(wl_shm::Format::Xbgr8888) => {
            Rgba([b0, b1, b2, 255])
        }
        // Unknown / unsupported formats: fall back to ARGB8888 byte layout (B,G,R,A).
        // This matches the most common wlroots setup, but colors will be wrong if the
        // compositor actually uses a different pixel format. We log here to make such
        // protocol / environment drift visible instead of failing hard.
        _ => {
            eprintln!(
                "read_pixel_rgba: unsupported wl_shm format {:?}, falling back to ARGB8888-like layout",
                format
            );
            Rgba([b2, b1, b0, 255])
        }
    }
}

/// Initialize a Wayland connection, discover globals, and resolve xdg-output names.
fn init_wayland() -> Result<(EventQueue<CaptureState>, CaptureState)> {
    let conn = Connection::connect_to_env()
        .map_err(|_| AppError::Other("Failed to connect to Wayland".to_string()))?;
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut state = CaptureState::new();
    let _registry = display.get_registry(&qh, ());

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| AppError::Other(format!("Wayland roundtrip failed: {e}")))?;

    let xdg_mgr = state
        .xdg_output_manager
        .as_ref()
        .ok_or_else(|| AppError::Other("zxdg_output_manager_v1 not available".to_string()))?
        .clone();

    for out in &mut state.outputs {
        out.xdg_output = Some(xdg_mgr.get_xdg_output(&out.output, &qh, ()));
    }

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| AppError::Other(format!("Wayland roundtrip failed: {e}")))?;

    Ok((event_queue, state))
}

// --- Dispatch impls ---

impl Dispatch<wl_registry::WlRegistry, ()> for CaptureState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match interface.as_str() {
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(name, 1, qh, ());
                    state.outputs.push(OutputInfo {
                        output,
                        name: None,
                        xdg_output: None,
                    });
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ()));
                }
                "zwlr_screencopy_manager_v1" => {
                    state.screencopy_manager = Some(
                        registry.bind::<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        ),
                    );
                }
                "zxdg_output_manager_v1" => {
                    state.xdg_output_manager = Some(
                        registry.bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1, _, _>(
                            name,
                            3,
                            qh,
                            (),
                        ),
                    );
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_shm::WlShm, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
        _: zwlr_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
        _: zxdg_output_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<zxdg_output_v1::ZxdgOutputV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        xdg_output: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_output_v1::Event::Name { name } = event {
            for out in &mut state.outputs {
                if out.xdg_output.as_ref() == Some(xdg_output) {
                    out.name = Some(name.clone());
                }
            }
        }
    }
}

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        frame: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let Some(fi_idx) = state.frames.iter().position(|f| &f.frame == frame) else {
            return;
        };

        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                let size = (stride * height) as usize;

                let memfd_name = CString::new("screencopy")
                    .expect("CString::new cannot fail for static literal");
                let fd = match memfd::memfd_create(&memfd_name, memfd::MemFdCreateFlag::MFD_CLOEXEC)
                {
                    Ok(fd) => fd,
                    Err(e) => {
                        state.frames[fi_idx].failed = true;
                        state.frames[fi_idx].error_msg = Some(format!("memfd_create failed: {e}"));
                        return;
                    }
                };

                if let Err(e) = nix::unistd::ftruncate(&fd, size as i64) {
                    state.frames[fi_idx].failed = true;
                    state.frames[fi_idx].error_msg = Some(format!("ftruncate failed: {e}"));
                    return;
                }

                let file = std::fs::File::from(fd);
                let mmap = match unsafe { MmapMut::map_mut(&file) } {
                    Ok(m) => m,
                    Err(e) => {
                        state.frames[fi_idx].failed = true;
                        state.frames[fi_idx].error_msg = Some(format!("mmap failed: {e}"));
                        return;
                    }
                };
                // Create the shm pool while only holding an immutable borrow of `state.shm`,
                // so that this borrow is finished before we later take a mutable borrow of
                // `state.frames[fi_idx]`. This keeps the lifecycle of Wayland shm resources
                // (pool and buffer) clearly separated from the mutation of per-frame state,
                // and ensures the shm global is available when setting up the frame buffer.
                let shm_format = match format {
                    WEnum::Value(v) => v,
                    WEnum::Unknown(raw) => {
                        state.frames[fi_idx].failed = true;
                        state.frames[fi_idx].error_msg =
                            Some(format!("unsupported shm format: {raw:#x}"));
                        return;
                    }
                };

                // Borrow shm separately; NLL ends this borrow before the mutable frame update below.
                let pool = match state.shm.as_ref() {
                    Some(shm) => shm.create_pool(file.as_fd(), size as i32, qh, ()),
                    None => {
                        state.frames[fi_idx].failed = true;
                        state.frames[fi_idx].error_msg =
                            Some("wl_shm global not available".to_string());
                        return;
                    }
                };

                let buffer = pool.create_buffer(
                    0,
                    width as i32,
                    height as i32,
                    stride as i32,
                    shm_format,
                    qh,
                    (),
                );
                pool.destroy();
                frame.copy(&buffer);

                let fi = &mut state.frames[fi_idx];
                fi.format = Some(WEnum::Value(shm_format));
                fi.width = width;
                fi.height = height;
                fi.stride = stride;
                fi.buffer = Some(buffer);
                fi.mmap = Some(mmap);
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                state.frames[fi_idx].ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                state.frames[fi_idx].failed = true;
                state.frames[fi_idx].error_msg =
                    Some("compositor rejected the screencopy request".to_string());
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_buffer::WlBuffer, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// --- Public capture API ---

/// Capture a single monitor at full physical resolution.
pub fn capture_monitor(monitor_name: &str) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let (mut event_queue, mut state) = init_wayland()?;
    let qh = event_queue.handle();

    let output = state
        .outputs
        .iter()
        .find(|o| o.name.as_deref() == Some(monitor_name))
        .ok_or_else(|| AppError::Other(format!("Monitor '{monitor_name}' not found")))?;

    let screencopy_mgr = state
        .screencopy_manager
        .as_ref()
        .ok_or_else(|| AppError::Other("zwlr_screencopy_manager_v1 not available".to_string()))?;

    let frame = screencopy_mgr.capture_output(0, &output.output, &qh, ());
    state.frames.push(FrameInfo {
        frame,
        width: 0,
        height: 0,
        stride: 0,
        format: None,
        ready: false,
        failed: false,
        error_msg: None,
        mmap: None,
        buffer: None,
        name: monitor_name.to_string(),
    });

    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .map_err(|e| AppError::Other(format!("Wayland dispatch failed: {e}")))?;
        let fi = &state.frames[0];
        if fi.ready || fi.failed {
            break;
        }
    }

    let fi = &state.frames[0];
    if fi.failed {
        return Err(AppError::Other(format!(
            "Screencopy failed for monitor '{monitor_name}': {}",
            fi.error_msg.as_deref().unwrap_or("unknown error"),
        )));
    }

    let mmap = fi.mmap.as_ref().ok_or_else(|| {
        AppError::Other("Screencopy buffer missing after ready signal".to_string())
    })?;
    let format = fi.format.ok_or_else(|| {
        AppError::Other("Screencopy format not set after ready signal".to_string())
    })?;
    let mut img = ImageBuffer::new(fi.width, fi.height);
    for y in 0..fi.height {
        for x in 0..fi.width {
            let offset = (y * fi.stride + x * 4) as usize;
            img.put_pixel(x, y, read_pixel_rgba(mmap, offset, format));
        }
    }
    Ok(img)
}

/// Capture all monitors and composite them into a single image in **logical pixel space**.
///
/// The output dimensions and pixel coordinates match what Hyprland IPC and slurp report,
/// so crop coordinates can be applied directly without coordinate conversion.
/// HiDPI monitors are downsampled to their logical size during compositing.
pub fn capture_all_monitors(
    monitors: &[crate::hyprland::MonitorInfo],
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    if monitors.is_empty() {
        return Err(AppError::Other(
            "No monitors provided to capture".to_string(),
        ));
    }

    let (mut event_queue, mut state) = init_wayland()?;
    let qh = event_queue.handle();

    let screencopy_mgr = state
        .screencopy_manager
        .as_ref()
        .ok_or_else(|| AppError::Other("zwlr_screencopy_manager_v1 not available".to_string()))?;

    // Compute bounding box and launch captures for all known monitors.
    let min_x = monitors
        .iter()
        .map(|m| m.rect.x)
        .min()
        .expect("monitors is non-empty, checked above");
    let min_y = monitors
        .iter()
        .map(|m| m.rect.y)
        .min()
        .expect("monitors is non-empty, checked above");
    let max_x = monitors
        .iter()
        .map(|m| m.rect.x + m.rect.w)
        .max()
        .expect("monitors is non-empty, checked above");
    let max_y = monitors
        .iter()
        .map(|m| m.rect.y + m.rect.h)
        .max()
        .expect("monitors is non-empty, checked above");

    for m in monitors {
        if let Some(output) = state
            .outputs
            .iter()
            .find(|o| o.name.as_deref() == Some(&m.name))
        {
            let frame = screencopy_mgr.capture_output(0, &output.output, &qh, ());
            state.frames.push(FrameInfo {
                frame,
                width: 0,
                height: 0,
                stride: 0,
                format: None,
                ready: false,
                failed: false,
                error_msg: None,
                mmap: None,
                buffer: None,
                name: m.name.clone(),
            });
        }
    }

    if state.frames.is_empty() {
        return Err(AppError::Other(
            "No matching Wayland outputs found for provided monitors".to_string(),
        ));
    }

    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .map_err(|e| AppError::Other(format!("Wayland dispatch failed: {e}")))?;
        if state.frames.iter().all(|f| f.ready || f.failed) {
            break;
        }
    }

    let failures: Vec<String> = state
        .frames
        .iter()
        .filter(|f| f.failed)
        .map(|f| {
            format!(
                "'{}': {}",
                f.name,
                f.error_msg.as_deref().unwrap_or("unknown error")
            )
        })
        .collect();
    if !failures.is_empty() {
        return Err(AppError::Other(format!(
            "Screencopy failed for: {}",
            failures.join(", ")
        )));
    }

    let total_width = (max_x - min_x).max(0) as u32;
    let total_height = (max_y - min_y).max(0) as u32;
    let mut master_img = ImageBuffer::new(total_width, total_height);

    for fi in &state.frames {
        let mmap = fi.mmap.as_ref().ok_or_else(|| {
            AppError::Other(format!(
                "Screencopy buffer missing for monitor '{}'",
                fi.name
            ))
        })?;
        let format = fi.format.ok_or_else(|| {
            AppError::Other(format!(
                "Screencopy format not set for monitor '{}'",
                fi.name
            ))
        })?;
        let mon_info = monitors
            .iter()
            .find(|m| m.name == fi.name)
            .ok_or_else(|| AppError::Other(format!("Monitor info missing for '{}'", fi.name)))?;

        let offset_x = (mon_info.rect.x - min_x) as u32;
        let offset_y = (mon_info.rect.y - min_y) as u32;
        let log_w = mon_info.rect.w as u32;
        let log_h = mon_info.rect.h as u32;

        // Pre-compute the logical→physical index mapping for each axis.
        // Using integer arithmetic (lx * fi.width / log_w) avoids per-pixel f64
        // operations and is exact for nearest-neighbour downsampling.
        // For 1x scale this degenerates to the identity mapping at zero cost.
        let phys_xs: Vec<u32> = (0..log_w)
            .map(|lx| (lx * fi.width / log_w).min(fi.width.saturating_sub(1)))
            .collect();
        let phys_ys: Vec<u32> = (0..log_h)
            .map(|ly| (ly * fi.height / log_h).min(fi.height.saturating_sub(1)))
            .collect();

        for (ly, &py) in phys_ys.iter().enumerate() {
            for (lx, &px) in phys_xs.iter().enumerate() {
                let offset = (py * fi.stride + px * 4) as usize;
                // lx < log_w (u32) and ly < log_h (u32), so usize → u32 never truncates.
                master_img.put_pixel(
                    offset_x + lx as u32,
                    offset_y + ly as u32,
                    read_pixel_rgba(mmap, offset, format),
                );
            }
        }
    }

    Ok(master_img)
}
