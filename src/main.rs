mod capture;
mod clipboard;
mod cmd;
mod config;
mod error;
mod freeze;
mod hyprland;
mod notify;

use clap::{Parser, Subcommand};
use config::Config;
use error::Result;

#[derive(Parser)]
#[command(name = "crop-hypr", about = "Hyprland screenshot tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Select a region with slurp and capture it
    Crop,
    /// Capture the active window (geometry via hyprctl)
    Window,
    /// Capture the focused monitor
    Monitor,
    /// Capture all monitors
    All,
    /// Freeze screen and select region interactively
    Freeze,
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Create config dir and load config
    let mut cfg = Config::load()?;

    // Create save directory during initialization
    std::fs::create_dir_all(&cfg.save_path)
        .map_err(|e| error::AppError::FileSystem(cfg.save_path.clone(), e))?;

    match cli.command {
        Commands::Crop => {
            if let Some(path) = capture::capture_crop(&cfg)? {
                finish(path)?;
            }
        }
        Commands::Window => finish(capture::capture_window(&cfg)?)?,
        Commands::Monitor => finish(capture::capture_monitor(&cfg)?)?,
        Commands::All => finish(capture::capture_all(&cfg)?)?,
        Commands::Freeze => {
            if let Some(path) = freeze::run_freeze(&cfg)? {
                finish(path)?;
            }
        }
    }

    Ok(())
}

fn finish(path: std::path::PathBuf) -> Result<()> {
    clipboard::copy_to_clipboard(&path)?;
    notify::notify_success(&path);
    println!("{}", path.display());
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        let _ = notify::notify_error(&e.to_string());
        std::process::exit(1);
    }
}
