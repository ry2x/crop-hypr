//! # platform::capture
//!
//! Screenshot capture implementations, one submodule per backend.
//!
//! | Module | Method | Use case |
//! |---|---|---|
//! | [`screencopy`] | `zwlr_screencopy_manager_v1` | Standard capture (crop / window / monitor / all / freeze) |
//! | [`portal`] | `xdg-desktop-portal` + PipeWire | `portal` subcommand — handles transparent windows |

pub mod portal;
pub mod screencopy;
