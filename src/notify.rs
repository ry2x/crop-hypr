use notify_rust::{Notification, Timeout};
use std::path::Path;
use std::process::Command;

use crate::config::NotificationsConfig;

pub fn notify_success(path: &Path, config: &NotificationsConfig) {
    if !config.enabled {
        return;
    }

    let path_str = path.display().to_string();
    let success_action = config.success_action.clone();

    match Notification::new()
        .appname("hyprcrop")
        .icon(&path_str)
        .summary(&config.success_summary.replace("{path}", &path_str))
        .body(&config.success_body.replace("{path}", &path_str))
        .action("default", "Open")
        .timeout(Timeout::Milliseconds(config.success_timeout))
        .show()
    {
        Ok(handle) => handle.wait_for_action(|action| {
            if action == "default" {
                let _ = Command::new(&success_action).arg(&path_str).spawn();
            }
        }),
        Err(e) => eprintln!("[hyprcrop] warning: failed to send notification: {e}"),
    }
}

pub fn notify_error(msg: &str, config: &NotificationsConfig) {
    if !config.enabled {
        return;
    }

    if let Err(e) = Notification::new()
        .appname("hyprcrop")
        .icon("dialog-error")
        .urgency(notify_rust::Urgency::Critical)
        .summary(&config.error_summary.replace("{error}", msg))
        .body(&config.error_body.replace("{error}", msg))
        .show()
    {
        eprintln!("[hyprcrop] warning: failed to send notification: {e}");
    }
}
