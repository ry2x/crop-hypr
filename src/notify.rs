use notify_rust::{Notification, Timeout};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use crate::config::Notifications;

pub fn notify_success(path: &Path, config: &Notifications) {
    if !config.enabled {
        return;
    }

    let path_str = path.display().to_string();
    let success_action = config.success_action.clone();
    let listen_for_action = config.success_timeout > 0;

    let mut notification = Notification::new();
    notification
        .appname("hyprcrop")
        .icon(&path_str)
        .summary(&config.success_summary.replace("{path}", &path_str))
        .body(&config.success_body.replace("{path}", &path_str))
        .timeout(Timeout::Never);

    if listen_for_action {
        notification.action("default", "Open");
    }

    let handle = match notification.show() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("[hyprcrop] warning: failed to send notification: {e}");
            return;
        }
    };

    if !listen_for_action {
        return;
    }

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        handle.wait_for_action(|action| {
            if action == "default" {
                let parts = match shell_words::split(&success_action) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!(
                            "[hyprcrop] error: failed to parse action command '{}': {e}",
                            success_action
                        );
                        return;
                    }
                };

                let Some((cmd, args)) = parts.split_first() else {
                    return;
                };

                let has_path_placeholder = parts.iter().any(|p| p.contains("{path}"));
                let cmd = cmd.replace("{path}", &path_str);
                let substituted: Vec<String> = args
                    .iter()
                    .map(|a| a.replace("{path}", &path_str))
                    .collect();

                let mut command = Command::new(&cmd);
                command.args(&substituted);
                if !has_path_placeholder {
                    command.arg(&path_str);
                }

                if let Err(e) = command.spawn() {
                    eprintln!(
                        "[hyprcrop] error: failed to execute action '{}': {e}",
                        success_action
                    );
                }
            }
        });
        let _ = tx.send(());
    });

    let _ = rx.recv_timeout(Duration::from_millis(config.success_timeout.into()));
}

pub fn notify_error(msg: &str, config: &Notifications) {
    if !config.enabled {
        return;
    }

    if let Err(e) = Notification::new()
        .appname("hyprcrop")
        .icon("dialog-error")
        .urgency(notify_rust::Urgency::Critical)
        .summary(&config.error_summary.replace("{error}", msg))
        .body(&config.error_body.replace("{error}", msg))
        .timeout(Timeout::Never)
        .show()
    {
        eprintln!("[hyprcrop] warning: failed to send notification: {e}");
    }
}
