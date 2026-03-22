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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Crop => {
            match capture::capture_crop() {
                Ok(Some(path)) => {
                    clipboard::copy_to_clipboard(&path)?;
                    notify::notify_success(&path);
                    println!("{}", path.display());
                }
                Ok(None) => {
                    // user cancelled slurp — exit silently
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    eprintln!("error: {msg}");
                    let _ = notify::notify_error(&msg);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
