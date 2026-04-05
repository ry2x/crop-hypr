use std::path::PathBuf;

use clap::{Parser, Subcommand};
use crop_hypr::config::Config;
use crop_hypr::error::{AppError, Result};
use crop_hypr::{capture, clipboard, freeze, notify};

#[derive(Parser)]
#[command(name = "crop-hypr", about = "Hyprland screenshot tool", version)]
struct Cli {
    /// Path to a custom config file (defaults to ~/.config/crop-hypr/config.toml)
    #[arg(long, short, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Select a region with slurp and capture it
    Crop,
    /// Capture the active window (geometry via hyprctl)
    Window,
    /// Capture the window via xdg-desktop-portal (not yet implemented)
    Portal,
    /// Capture the focused monitor
    Monitor,
    /// Capture all monitors
    All,
    /// Freeze screen and select region interactively
    Freeze,
    /// Write a default config.toml to ~/.config/crop-hypr/config.toml (or --config path)
    GenerateConfig {
        /// Overwrite the file if it already exists
        #[arg(long)]
        force: bool,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if let Commands::GenerateConfig { force } = cli.command {
        return generate_config(cli.config.as_deref(), force);
    }

    let cfg = match &cli.config {
        Some(path) => Config::load_from(path)?,
        None => Config::load()?,
    };

    // Create save directory during initialization
    std::fs::create_dir_all(&cfg.save_path)
        .map_err(|e| AppError::FileSystem(cfg.save_path.clone(), e))?;

    match cli.command {
        Commands::Crop => finish(capture::capture_crop(&cfg)?)?,
        Commands::Window => finish(capture::capture_window(&cfg)?)?,
        Commands::Portal => finish(capture::capture_portal(&cfg)?)?,
        Commands::Monitor => finish(capture::capture_monitor(&cfg)?)?,
        Commands::All => finish(capture::capture_all(&cfg)?)?,
        Commands::Freeze => finish(freeze::run_freeze(&cfg)?)?,
        Commands::GenerateConfig { .. } => unreachable!(),
    }

    Ok(())
}

fn generate_config(custom_path: Option<&std::path::Path>, force: bool) -> Result<()> {
    let path = custom_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(Config::default_config_path);

    if path.exists() && !force {
        return Err(AppError::Config(format!(
            "config file already exists: {}\nUse --force to overwrite",
            path.display()
        )));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::FileSystem(parent.to_path_buf(), e))?;
    }

    let content = Config::generate_default_toml()?;
    std::fs::write(&path, &content).map_err(|e| AppError::FileSystem(path.clone(), e))?;

    println!("config written to: {}", path.display());
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
        if let AppError::UserCancelled = &e {
            std::process::exit(e.exit_code());
        }

        eprintln!("error: {}", e);
        let _ = notify::notify_error(&e.to_string());
        std::process::exit(e.exit_code());
    }
}
