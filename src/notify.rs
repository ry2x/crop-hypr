use notify_rust::{Notification, Timeout};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use crate::config::NotificationsConfig;

pub fn notify_success(path: &Path, config: &NotificationsConfig) {
    if !config.enabled {
        return;
    }

    let path_str = path.display().to_string();
    let success_action = config.success_action.clone();

    let handle = match Notification::new()
        .appname("hyprcrop")
        .icon(&path_str)
        .summary(&config.success_summary.replace("{path}", &path_str))
        .body(&config.success_body.replace("{path}", &path_str))
        .action("default", "Open")
        .timeout(Timeout::Never)
        .show()
    {
        Ok(h) => h,
        Err(e) => {
            eprintln!("[hyprcrop] warning: failed to send notification: {e}");
            return;
        }
    };

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        handle.wait_for_action(|action| {
            if action == "default" {
                let _ = Command::new(&success_action).arg(&path_str).spawn();
            }
        });
        let _ = tx.send(());
    });

    let _ = rx.recv_timeout(Duration::from_millis(config.success_timeout.into()));
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
