use std::{fs, path::PathBuf};

use crate::freeze::CaptureMode;

/// Returns `$XDG_STATE_HOME/crop-hypr/last_mode`
/// (falls back to `~/.local/state/crop-hypr/last_mode`).
fn state_file() -> Option<PathBuf> {
    dirs::state_dir().map(|d| d.join("crop-hypr").join("last_mode"))
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
        "all" => None,
        _ => None,
    }
}

/// Persist the last-used freeze mode. `CaptureMode::All` is intentionally
/// excluded — it fires immediately on press so there is no meaningful "last
/// interactive mode" to remember.
pub fn save_last_mode(mode: CaptureMode) {
    if mode == CaptureMode::All {
        return;
    }
    let Some(path) = state_file() else { return };
    if let Some(parent) = path.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return;
    }
    let _ = fs::write(&path, mode_to_str(mode));
}

/// Load the last-used freeze mode from disk.
/// Returns `CaptureMode::Crop` if the file is missing, unreadable, or contains
/// an unrecognised value.
pub fn load_last_mode() -> CaptureMode {
    let Some(path) = state_file() else {
        return CaptureMode::Crop;
    };
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| str_to_mode(&s))
        .unwrap_or(CaptureMode::Crop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Serialise all env-var-mutating tests to prevent races under `cargo test`
    // (which runs tests in parallel by default).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_tmp_state<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        // SAFETY: tests run sequentially under ENV_LOCK; no concurrent
        // threads mutate XDG_STATE_HOME while the lock is held.
        unsafe {
            env::set_var("XDG_STATE_HOME", tmp.path());
        }
        f();
        unsafe {
            env::remove_var("XDG_STATE_HOME");
        }
    }

    #[test]
    fn test_round_trip_crop() {
        with_tmp_state(|| {
            save_last_mode(CaptureMode::Crop);
            assert_eq!(load_last_mode(), CaptureMode::Crop);
        });
    }

    #[test]
    fn test_round_trip_window() {
        with_tmp_state(|| {
            save_last_mode(CaptureMode::Window);
            assert_eq!(load_last_mode(), CaptureMode::Window);
        });
    }

    #[test]
    fn test_round_trip_monitor() {
        with_tmp_state(|| {
            save_last_mode(CaptureMode::Monitor);
            assert_eq!(load_last_mode(), CaptureMode::Monitor);
        });
    }

    #[test]
    fn test_all_is_not_saved() {
        with_tmp_state(|| {
            // First save something meaningful
            save_last_mode(CaptureMode::Window);
            // Then "save" All — should be a no-op
            save_last_mode(CaptureMode::All);
            assert_eq!(load_last_mode(), CaptureMode::Window);
        });
    }

    #[test]
    fn test_missing_file_returns_crop() {
        with_tmp_state(|| {
            assert_eq!(load_last_mode(), CaptureMode::Crop);
        });
    }

    #[test]
    fn test_unknown_content_returns_crop() {
        with_tmp_state(|| {
            let path = state_file().unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "bogus").unwrap();
            assert_eq!(load_last_mode(), CaptureMode::Crop);
        });
    }
}
