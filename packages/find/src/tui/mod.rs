//! The interactive picker (port of zehn's `src/tui.zig`).
//!
//! CDXC:PromptSearch 2026-08-20:
//! Two hotkeys moved so the terminal picker and the Find GUI can share one key
//! map. `^t` (agents) and `^r` (projects) are unusable in the GUI: browsers
//! reserve Ctrl+T for a new tab and Ctrl+R for reload, and a page cannot take
//! them back. They are now `^g` (a-g-ents) and `^j` (pro-j-ect) in both surfaces.
//! Because `^j` is byte 10, Enter is CR-only here — raw mode already clears
//! ICRNL, so Enter always arrives as byte 13.

mod editing;
mod format;
mod input;
mod picker;
mod render;
mod terminal;
#[cfg(test)]
mod tests;

pub use format::*;
pub use picker::*;
use terminal::*;
