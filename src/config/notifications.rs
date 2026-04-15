use serde::{Deserialize, Serialize};

// ── NotificationsConfig ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub enable: bool,
    pub success_action: String,
    pub success_timeout: u32,
    pub success_summary: String,
    pub success_body: String,
    pub error_summary: String,
    pub error_body: String,
}

// Available variables for notification bodies:
// - {path}: the path to the saved screenshot (for success_summary and success_body)
// - {error}: the error message (for error_summary and error_body)
// success_timeout is in milliseconds. Set to 0 for no timeout (persistent notification).
// default success_timeout is 5000 (5 seconds). Note that some desktop environments may have a maximum timeout limit, and may ignore timeouts that exceed this limit.
// In general, longer timeouts are recommended for the success notification, since it allows the user more time to click the notification themselves.
impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enable: true,
            success_action: "xdg-open".to_string(),
            success_timeout: 5000,
            success_summary: "Screenshot saved".to_string(),
            success_body: "{path}".to_string(),
            error_summary: "Screenshot failed".to_string(),
            error_body: "{error}".to_string(),
        }
    }
}
