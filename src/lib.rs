//! Rar N Rush — watches a download folder, verifies PAR2/SFV sets, repairs them
//! and extracts RAR archives automatically.
//!
//! The crate is split into a library (everything testable) and a thin binary
//! (`src/main.rs`) that only starts the GUI.
//!
//! Module overview:
//! - [`scan`]: discovers file sets (PAR2, SFV, RAR volumes) in a folder.
//! - [`par2`]: runs an external `par2` executable to verify and repair sets.
//! - [`sfv`]: verifies CRC32 checksums listed in SFV files.
//! - [`extract`]: extracts RAR archives with the official `UnRAR` library.
//! - [`engine`]: background worker that ties everything together.
//! - [`settings`]: user settings and persistent state.
//! - [`i18n`]: user-facing messages in English and French.
//! - [`ui`]: the egui desktop interface.

pub mod engine;
pub mod extract;
pub mod i18n;
pub mod icon;
pub mod par2;
pub mod scan;
pub mod secrets;
pub mod settings;
pub mod sfv;
mod sync;
pub mod tidy;
pub mod tray;
pub mod ui;
