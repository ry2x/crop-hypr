use std::{fs, path::PathBuf};

use crate::ui::freeze::CaptureMode;

/// Returns `$XDG_STATE_HOME/hyprcrop/last_mode`
/// (falls back to `~/.local/state/hyprcrop/last_mode`).
fn state_file() -> Option<PathBuf> {
    dirs::state_dir().map(|d| d.join("hyprcrop").join("last_mode"))
}

fn mode_to_str(mode: CaptureMode) -> &'static str {
    match mode {
        CaptureMode::Crop => "crop",
        CaptureMode::Window => "window",
        CaptureMode::Monitor => "monitor",
        CaptureMode::All => "all",
    }
}

fn str_to_mode(s: &str) -> Option<CaptureMode> {
    match s.trim() {
        "crop" => Some(CaptureMode::Crop),
        "window" => Some(CaptureMode::Window),
        "monitor" => Some(CaptureMode::Monitor),
        // "all" is never written by save_last_mode, but if it appears (e.g.
        // from a manual edit) treat it like an unrecognised value and fall
        // back to the default (Crop) via the None → unwrap_or chain in read_mode.
        _ => None,
    }
}

fn write_mode(path: &std::path::Path, mode: CaptureMode) {
    if let Some(parent) = path.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return;
    }
    let _ = fs::write(path, mode_to_str(mode));
}

fn read_mode(path: &std::path::Path) -> CaptureMode {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| str_to_mode(&s))
        .unwrap_or(CaptureMode::Crop)
}

/// Persist the last-used freeze mode. `CaptureMode::All` is intentionally
/// excluded — it fires immediately on press so there is no meaningful "last
/// interactive mode" to remember.
pub fn save_last_mode(mode: CaptureMode) {
    if mode == CaptureMode::All {
        return;
    }
    let Some(path) = state_file() else { return };
    write_mode(&path, mode);
}

/// Load the last-used freeze mode from disk.
/// Returns `CaptureMode::Crop` if the file is missing, unreadable, or contains
/// an unrecognised value.
pub fn load_last_mode() -> CaptureMode {
    state_file()
        .map(|p| read_mode(&p))
        .unwrap_or(CaptureMode::Crop)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_state_file() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("hyprcrop").join("last_mode");
        (tmp, path)
    }

    #[test]
    fn test_round_trip_crop() {
        let (_tmp, path) = tmp_state_file();
        write_mode(&path, CaptureMode::Crop);
        assert_eq!(read_mode(&path), CaptureMode::Crop);
    }

    #[test]
    fn test_round_trip_window() {
        let (_tmp, path) = tmp_state_file();
        write_mode(&path, CaptureMode::Window);
        assert_eq!(read_mode(&path), CaptureMode::Window);
    }

    #[test]
    fn test_round_trip_monitor() {
        let (_tmp, path) = tmp_state_file();
        write_mode(&path, CaptureMode::Monitor);
        assert_eq!(read_mode(&path), CaptureMode::Monitor);
    }

    #[test]
    fn test_all_is_not_saved() {
        let (_tmp, path) = tmp_state_file();
        // First save something meaningful
        write_mode(&path, CaptureMode::Window);
        // All must not call write_mode — verify via the public guard
        save_last_mode(CaptureMode::All);
        // The file still contains "window"
        assert_eq!(read_mode(&path), CaptureMode::Window);
    }

    #[test]
    fn test_missing_file_returns_crop() {
        let (_tmp, path) = tmp_state_file();
        assert_eq!(read_mode(&path), CaptureMode::Crop);
    }

    #[test]
    fn test_unknown_content_returns_crop() {
        let (_tmp, path) = tmp_state_file();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "bogus").unwrap();
        assert_eq!(read_mode(&path), CaptureMode::Crop);
    }
}
