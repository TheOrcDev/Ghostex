//! The browser build's copy of the desktop's `native_chat/mermaid_render.rs`.
//!
//! CDXC:SessionChat 2026-09-30 WHY: mermaid-rs-renderer times its layout passes with `std::time::Instant`, which panics on wasm32-unknown-unknown, so the page cannot draw a diagram; it shows the diagram's source with this note, and has no larger viewer to open.

use gpui::RenderImage;
use std::sync::Arc;

pub(super) fn svg(_source: &str, _light: bool) -> Result<(Arc<str>, f32, f32), String> {
    Err("Diagrams are drawn in the Ghostex app. This is the diagram’s source.".to_string())
}

pub(super) fn raster(_svg: &str, _scale: f32) -> Option<Arc<RenderImage>> {
    None
}

pub(super) const HAS_VIEWER: bool = false;
