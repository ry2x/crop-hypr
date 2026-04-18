//! # domain
//!
//! Domain model shared across the entire application.
//! Contains no external I/O — only pure data definitions, calculations, and configuration parsing.
//!
//! ## Submodules
//!
//! | Module | Contents |
//! |---|---|
//! | [`config`] | TOML configuration schema, loading, and default values |
//! | [`error`] | Application-wide error type `AppError` and `Result<T>` alias |
//! | [`geometry`] | Coordinate calculations (slurp string parsing, logical-to-physical conversion, clamping) |
//! | [`state`] | Persists the last-used capture mode for freeze mode across sessions |
//! | [`types`] | Shared data structures: `ScreenRect`, `MonitorInfo`, `WindowInfo`, etc. |

pub mod config;
pub mod error;
pub mod geometry;
pub mod state;
pub mod types;
