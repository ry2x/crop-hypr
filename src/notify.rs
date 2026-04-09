use std::path::Path;

use crate::cmd::{CMD_NOTIFY_SEND, run_cmd_status};
use crate::error::Result;

pub fn notify_success(path: &Path) {
    let path_str = path.display().to_string();
    let _ = run_cmd_status(
        CMD_NOTIFY_SEND,
        [
            "--app-name=hyprcrop",
            &format!("--icon={path_str}"),
            "Screenshot saved",
            &path_str,
        ],
    );
}

pub fn notify_error(msg: &str) -> Result<()> {
    run_cmd_status(
        CMD_NOTIFY_SEND,
        [
            "--app-name=hyprcrop",
            "--icon=dialog-error",
            "--urgency=critical",
            "Screenshot failed",
            msg,
        ],
    )?;
    Ok(())
}
