//! InputFlow core engine — platform independent, no I/O, fully unit-tested.
//!
//! Native integrations live in the Tauri app (`src-tauri`) behind platform
//! adapters; this crate holds the logic that must behave identically on
//! macOS and Windows.

pub mod actions;
pub mod calc;
pub mod devtools;
pub mod launcher;
pub mod menu;
pub mod settings;
pub mod snippets;
pub mod text;
pub mod units;
pub mod writing;
