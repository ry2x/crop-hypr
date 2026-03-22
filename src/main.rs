mod capture;
mod clipboard;
mod notify;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Crop => {
            if let Some(path) = capture::capture_crop()? {
                finish(path)?;
            }
        }
        Commands::Window => finish(capture::capture_window()?)?,
        Commands::Monitor => finish(capture::capture_monitor()?)?,
        Commands::All => finish(capture::capture_all()?)?,
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
        let msg = format!("{e:#}");
        eprintln!("error: {msg}");
        let _ = notify::notify_error(&msg);
        std::process::exit(1);
    }
}
