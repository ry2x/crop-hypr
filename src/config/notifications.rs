use serde::{Deserialize, Serialize};

// ── Notifications ───────────────────────────────────────────────────────────────

fn default_enabled() -> bool {
    true
}
fn default_success_action() -> String {
    "xdg-open".to_string()
}
fn default_success_timeout() -> u32 {
    5000
}
fn default_success_summary() -> String {
    "Screenshot saved".to_string()
}
fn default_success_body() -> String {
    "{path}".to_string()
}
fn default_error_summary() -> String {
    "Screenshot failed".to_string()
}
fn default_error_body() -> String {
    "{error}".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notifications {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_success_action")]
    pub success_action: String,
    #[serde(default = "default_success_timeout")]
    pub success_timeout: u32,
    #[serde(default = "default_success_summary")]
    pub success_summary: String,
    #[serde(default = "default_success_body")]
    pub success_body: String,
    #[serde(default = "default_error_summary")]
    pub error_summary: String,
    #[serde(default = "default_error_body")]
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
            enabled: default_enabled(),
            success_action: default_success_action(),
            success_timeout: default_success_timeout(),
            success_summary: default_success_summary(),
            success_body: default_success_body(),
            error_summary: default_error_summary(),
            error_body: default_error_body(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let n = Notifications::default();
        assert!(n.enabled);
        assert_eq!(n.success_action, "xdg-open");
        assert_eq!(n.success_timeout, 5000);
        assert_eq!(n.success_summary, "Screenshot saved");
        assert_eq!(n.success_body, "{path}");
        assert_eq!(n.error_summary, "Screenshot failed");
        assert_eq!(n.error_body, "{error}");
    }

    #[test]
    fn toml_empty_uses_defaults() {
        let n: Notifications = toml::from_str("").unwrap();
        assert_eq!(n.enabled, Notifications::default().enabled);
        assert_eq!(n.success_timeout, Notifications::default().success_timeout);
        assert_eq!(n.success_action, Notifications::default().success_action);
    }

    #[test]
    fn toml_partial_override_falls_back_to_defaults() {
        let n: Notifications = toml::from_str("success_timeout = 3000").unwrap();
        assert_eq!(n.success_timeout, 3000);
        assert_eq!(n.success_action, Notifications::default().success_action);
        assert_eq!(n.success_summary, Notifications::default().success_summary);
        assert_eq!(n.error_summary, Notifications::default().error_summary);
    }

    #[test]
    fn toml_disabled() {
        let n: Notifications = toml::from_str("enabled = false").unwrap();
        assert!(!n.enabled);
    }

    #[test]
    fn toml_zero_timeout_fire_and_forget() {
        let n: Notifications = toml::from_str("success_timeout = 0").unwrap();
        assert_eq!(n.success_timeout, 0);
    }

    #[test]
    fn toml_full_override() {
        let toml = r#"
            enabled = false
            success_action = "eog"
            success_timeout = 8000
            success_summary = "Done"
            success_body = "Saved to {path}"
            error_summary = "Oops"
            error_body = "{error} occurred"
        "#;
        let n: Notifications = toml::from_str(toml).unwrap();
        assert!(!n.enabled);
        assert_eq!(n.success_action, "eog");
        assert_eq!(n.success_timeout, 8000);
        assert_eq!(n.success_summary, "Done");
        assert_eq!(n.success_body, "Saved to {path}");
        assert_eq!(n.error_summary, "Oops");
        assert_eq!(n.error_body, "{error} occurred");
    }
}
