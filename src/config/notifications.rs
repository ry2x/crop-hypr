use serde::{Deserialize, Serialize};

// ── Notifications ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notifications {
    pub enabled: bool,
    pub success_action: String,
    pub success_timeout: u32,
    pub success_summary: String,
    pub success_body: String,
    pub error_summary: String,
    pub error_body: String,
}

// Available variables:
// - {path}: path to the saved screenshot (success_summary, success_body)
// - {error}: error message (error_summary, error_body)
//
// success_timeout (milliseconds): how long hyprcrop waits for the user to click "Open".
// Set to 0 for fire-and-forget — process exits immediately, no action button is shown.
// Note: notifications always persist on screen until dismissed by the user.
impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            success_action: "xdg-open".to_string(),
            success_timeout: 5000,
            success_summary: "Screenshot saved".to_string(),
            success_body: "{path}".to_string(),
            error_summary: "Screenshot failed".to_string(),
            error_body: "{error}".to_string(),
        }
    }
}
