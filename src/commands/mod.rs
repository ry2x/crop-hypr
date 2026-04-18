//! # commands
//!
//! CLI subcommand implementations. Each subcommand takes a `Config` and returns
//! the path to the captured image (`PathBuf`).
//! Post-processing (clipboard copy and notification) is handled centrally by `finish()` in `main.rs`.
//!
//! ## Subcommands
//!
//! | Subcommand | Behavior |
//! |---|---|
//! | `crop` | Captures a region selected interactively via `slurp` |
//! | `window` | Captures the active window (geometry or portal method) |
//! | `monitor` | Captures the focused monitor |
//! | `all` | Captures all monitors and stitches them into one image |

pub mod capture;
