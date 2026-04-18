//! # ui
//!
//! Interactive overlay UI for freeze mode.
//! Built with `iced` + `iced_layershell` on a Wayland layer-shell surface at overlay level.
//!
//! ## Responsibilities
//!
//! - Renders the frozen full-screen screenshot as an overlay
//! - Presents a toolbar (crop / window / monitor / all / cancel) for capture mode selection
//! - Crops the frozen image to the selected region and produces the final output

pub mod freeze;
