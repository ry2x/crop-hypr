use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Command not found or failed to spawn: {0}")]
    CommandNotFound(String, #[source] std::io::Error),

    #[error("Command {0} failed with exit status: {1}")]
    CommandFailed(String, std::process::ExitStatus),

    #[error("Hyprland IPC error: {0}")]
    HyprlandIpc(#[from] std::io::Error),

    #[error("Hyprland IPC environment variable missing: {0}")]
    HyprlandEnvVar(#[from] std::env::VarError),

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Failed to load or parse TOML config: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("User cancelled operation")]
    UserCancelled,

    #[error("Slurp returned empty geometry")]
    EmptyGeometry,

    #[error("No focused monitor found")]
    NoFocusedMonitor,

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Iced Layershell error")]
    LayerShell,

    #[error("File system error on path {0}: {1}")]
    FileSystem(PathBuf, #[source] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
