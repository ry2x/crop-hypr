use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::cmd::CMD_SLURP;
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::hyprland;
use crate::screencopy;

fn slurp_region() -> Result<String> {
    let output = Command::new(CMD_SLURP)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| AppError::CommandNotFound(CMD_SLURP.to_string(), e))?;

    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Err(AppError::UserCancelled);
        }
        return Err(AppError::CommandFailed(
            CMD_SLURP.to_string(),
            output.status,
        ));
    }

    let region = String::from_utf8(output.stdout)
        .map_err(|_| AppError::Other("slurp output is not valid UTF-8".to_string()))?
        .trim()
        .to_owned();

    if region.is_empty() {
        return Err(AppError::EmptyGeometry);
    }

    Ok(region)
}

/// Parse a slurp geometry string (`"X,Y WxH"`) into `(x, y, w, h)`.
///
/// `x` and `y` are signed because multi-monitor setups can place monitors at
/// negative logical coordinates (e.g. a monitor to the left of the primary at `−1920,0`).
fn parse_slurp_geometry(geom: &str) -> Result<(i32, i32, u32, u32)> {
    let (pos_part, size_part) = geom
        .trim()
        .split_once(' ')
        .ok_or_else(|| AppError::Other("Invalid geometry string: missing space".to_string()))?;

    let (x_str, y_str) = pos_part.trim().split_once(',').ok_or_else(|| {
        AppError::Other("Invalid geometry string: missing comma in position".to_string())
    })?;

    let (w_str, h_str) = size_part.trim().split_once('x').ok_or_else(|| {
        AppError::Other("Invalid geometry string: missing 'x' in size".to_string())
    })?;

    let x = x_str
        .trim()
        .parse::<i32>()
        .map_err(|_| AppError::Other(format!("Invalid x in geometry: '{}'", x_str)))?;
    let y = y_str
        .trim()
        .parse::<i32>()
        .map_err(|_| AppError::Other(format!("Invalid y in geometry: '{}'", y_str)))?;
    let w = w_str
        .trim()
        .parse::<u32>()
        .map_err(|_| AppError::Other(format!("Invalid w in geometry: '{}'", w_str)))?;
    let h = h_str
        .trim()
        .parse::<u32>()
        .map_err(|_| AppError::Other(format!("Invalid h in geometry: '{}'", h_str)))?;

    Ok((x, y, w, h))
}

/// Clamp a crop rectangle to image bounds.
///
/// Returns `(clamped_w, clamped_h, was_clamped)`.
/// `was_clamped` is `true` when the requested size exceeded the image, which
/// callers should surface as a warning — silent clamping hides crop mistakes.
pub fn clamp_crop(x: u32, y: u32, w: u32, h: u32, img_w: u32, img_h: u32) -> (u32, u32, bool) {
    let clamped_w = w.min(img_w.saturating_sub(x));
    let clamped_h = h.min(img_h.saturating_sub(y));
    (clamped_w, clamped_h, clamped_w != w || clamped_h != h)
}

/// Scale a single logical-space coordinate or dimension to physical pixels.
///
/// Rounding behavior:
/// - We intentionally use floor semantics (never round up) so that scaled
///   coordinates/dimensions do not overshoot the framebuffer by one pixel on
///   HiDPI setups.
/// - `logical` is non-negative (`u32`), so `floor` is equivalent to truncation
///   here; this function documents that choice explicitly.
fn logical_to_physical(logical: u32, scale: f64) -> u32 {
    // Use explicit floor to make the truncation semantics and HiDPI behavior clear.
    (f64::from(logical) * scale).floor() as u32
}

pub fn capture_crop(cfg: &Config) -> Result<PathBuf> {
    // Fetch monitor layout before blocking on slurp so the layout snapshot used to
    // interpret slurp's logical coordinates stays stable while the user selects.
    let monitors = hyprland::parse_monitors(hyprland::get_monitors()?);
    let region = slurp_region()?;
    let (slurp_x, slurp_y, req_w, req_h) = parse_slurp_geometry(&region)?;

    // capture_all_monitors places (min_x, min_y) at image pixel (0, 0).
    // Slurp returns global logical coordinates, so we must subtract the origin
    // to get image-space coordinates. This matters for multi-monitor layouts
    // where some monitors sit at negative logical positions.
    let min_x = monitors.iter().map(|m| m.rect.x).min().unwrap_or(0);
    let min_y = monitors.iter().map(|m| m.rect.y).min().unwrap_or(0);
    let x = (slurp_x - min_x).max(0) as u32;
    let y = (slurp_y - min_y).max(0) as u32;

    let full_img = screencopy::capture_all_monitors(&monitors)?;

    let (w, h, was_clamped) = clamp_crop(x, y, req_w, req_h, full_img.width(), full_img.height());
    if was_clamped {
        eprintln!(
            "warning: crop region ({slurp_x},{slurp_y} {req_w}x{req_h}) exceeds image bounds ({}x{}), clamped to {w}x{h}",
            full_img.width(),
            full_img.height(),
        );
    }
    if w == 0 || h == 0 {
        return Err(AppError::Other(format!(
            "Crop region ({slurp_x},{slurp_y} {req_w}x{req_h}) is entirely outside the image bounds ({}x{})",
            full_img.width(),
            full_img.height(),
        )));
    }

    let cropped = ::image::imageops::crop_imm(&full_img, x, y, w, h).to_image();
    let path = cfg.output_path();
    cropped.save(&path).map_err(AppError::from)?;
    Ok(path)
}

pub fn capture_window(cfg: &Config) -> Result<PathBuf> {
    let info = hyprland::get_active_window()?;

    let border_size = if cfg.capture_window_border {
        hyprland::get_border_style().border_size as i32
    } else {
        0
    };

    // Keep window position as i32: coordinates can be negative for off-screen windows.
    // Expand by border_size on each side when requested.
    let win_x = info.at[0] - border_size;
    let win_y = info.at[1] - border_size;
    let win_w = (info.size[0] + 2 * border_size).max(0) as u32;
    let win_h = (info.size[1] + 2 * border_size).max(0) as u32;

    // Identify the monitor that contains the window's top-left corner.
    // Windows spanning multiple monitors are captured from the monitor containing their top-left corner only.
    let monitors = hyprland::parse_monitors(hyprland::get_monitors()?);
    let mon = monitors
        .iter()
        .find(|m| {
            win_x >= m.rect.x
                && win_y >= m.rect.y
                && win_x < m.rect.x + m.rect.w
                && win_y < m.rect.y + m.rect.h
        })
        .ok_or_else(|| AppError::Other("Could not find monitor for active window".to_string()))?;

    let mon_img = screencopy::capture_monitor(&mon.name)?;

    // Derive scale from actual frame dimensions (handles HiDPI without a separate field).
    if mon.rect.w <= 0 || mon.rect.h <= 0 {
        return Err(AppError::Other(format!(
            "Monitor '{}' has invalid dimensions ({}x{}) in Hyprland IPC data",
            mon.name, mon.rect.w, mon.rect.h
        )));
    }
    let scale_x = f64::from(mon_img.width()) / f64::from(mon.rect.w);
    let scale_y = f64::from(mon_img.height()) / f64::from(mon.rect.h);

    // Window position relative to monitor top-left, clamped to non-negative.
    let rel_x = (win_x - mon.rect.x).max(0) as u32;
    let rel_y = (win_y - mon.rect.y).max(0) as u32;

    // Convert logical → physical pixels, then clamp to frame bounds.
    let phys_x = logical_to_physical(rel_x, scale_x);
    let phys_y = logical_to_physical(rel_y, scale_y);
    let phys_w = logical_to_physical(win_w, scale_x).min(mon_img.width().saturating_sub(phys_x));
    let phys_h = logical_to_physical(win_h, scale_y).min(mon_img.height().saturating_sub(phys_y));

    if phys_w == 0 || phys_h == 0 {
        return Err(AppError::Other(
            "Window crop region is entirely outside the monitor image bounds".to_string(),
        ));
    }

    let cropped = ::image::imageops::crop_imm(&mon_img, phys_x, phys_y, phys_w, phys_h).to_image();
    let path = cfg.output_path();
    cropped.save(&path).map_err(AppError::from)?;
    Ok(path)
}

pub fn capture_monitor(cfg: &Config) -> Result<PathBuf> {
    let monitors = hyprland::get_monitors()?;
    let focused = monitors
        .into_iter()
        .find(|m| m.focused)
        .ok_or(AppError::NoFocusedMonitor)?;

    let img = screencopy::capture_monitor(&focused.name)?;
    let path = cfg.output_path();
    img.save(&path).map_err(AppError::from)?;
    Ok(path)
}

pub fn capture_portal(cfg: &Config) -> Result<PathBuf> {
    crate::portal::capture(cfg)
}

pub fn capture_all(cfg: &Config) -> Result<PathBuf> {
    let monitors = hyprland::parse_monitors(hyprland::get_monitors()?);
    let img = screencopy::capture_all_monitors(&monitors)?;

    let path = cfg.output_path();
    img.save(&path).map_err(AppError::from)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slurp_geometry() {
        // Standard geometry
        let (x, y, w, h) = parse_slurp_geometry("10,20 100x200").unwrap();
        assert_eq!(x, 10);
        assert_eq!(y, 20);
        assert_eq!(w, 100);
        assert_eq!(h, 200);

        // Negative coordinates
        let (x, y, w, h) = parse_slurp_geometry("-1920,0 1920x1080").unwrap();
        assert_eq!(x, -1920);
        assert_eq!(y, 0);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);

        // Extra whitespace (surrounding)
        let (x, y, w, h) = parse_slurp_geometry("  10,20 100x200  ").unwrap();
        assert_eq!(x, 10);
        assert_eq!(y, 20);
        assert_eq!(w, 100);
        assert_eq!(h, 200);

        // Invalid string (completely unparseable)
        assert!(parse_slurp_geometry("invalid").is_err());

        // Invalid string (missing size)
        assert!(parse_slurp_geometry("10,20 100").is_err());

        // Invalid string (extra commas)
        assert!(parse_slurp_geometry("10,20, 100x200").is_err());

        // Invalid string (missing coordinate)
        assert!(parse_slurp_geometry("10 100x200").is_err());

        // Invalid string (non-numeric parts)
        assert!(parse_slurp_geometry("a,b cxd").is_err());
    }

    #[test]
    fn test_clamp_crop() {
        // No clamping needed
        let (w, h, clamped) = clamp_crop(0, 0, 100, 100, 200, 200);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
        assert!(!clamped);

        // Clamping width
        let (w, h, clamped) = clamp_crop(150, 0, 100, 100, 200, 200);
        assert_eq!(w, 50);
        assert_eq!(h, 100);
        assert!(clamped);

        // Clamping height
        let (w, h, clamped) = clamp_crop(0, 150, 100, 100, 200, 200);
        assert_eq!(w, 100);
        assert_eq!(h, 50);
        assert!(clamped);

        // Out of bounds entirely
        let (w, h, clamped) = clamp_crop(250, 250, 100, 100, 200, 200);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
        assert!(clamped);

        // Exact boundary (touching right edge)
        let (w, h, clamped) = clamp_crop(100, 0, 100, 100, 200, 200);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
        assert!(!clamped);

        // Overlapping boundary (starts inside, exceeds right edge)
        let (w, h, clamped) = clamp_crop(150, 0, 100, 100, 200, 200);
        assert_eq!(w, 50);
        assert_eq!(h, 100);
        assert!(clamped);
    }

    #[test]
    fn test_logical_to_physical() {
        // 1x scale
        assert_eq!(logical_to_physical(100, 1.0), 100);

        // 2x scale (HiDPI)
        assert_eq!(logical_to_physical(100, 2.0), 200);

        // Fractional scale (1.5x)
        assert_eq!(logical_to_physical(100, 1.5), 150);

        // Floor behavior
        // 100 * 1.25 = 125.0
        assert_eq!(logical_to_physical(100, 1.25), 125);

        // 101 * 1.25 = 126.25 -> 126
        assert_eq!(logical_to_physical(101, 1.25), 126);
    }
}
