//! The phone's copy of the desktop's `native_chat/mermaid_render.rs`: the Docs view's renderer,
//! lifted from `native_docs/blocks.rs` by `extracted-items.txt`, with the phone's own fonts.
//!
//! CDXC:Mobile 2026-09-30 WHY: fontdb's `load_system_fonts` finds no fonts on Android or iOS, so the desktop's rasterizer drew a diagram's boxes and arrows with no text on the phone. This `fontdb` reads the platform's font folder and points the SVG's `sans-serif` at a face that is there. Without system fonts the renderer sizes its labels from its own width estimates, which the drawn text fits.

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use std::sync::{Arc, OnceLock};

include!(concat!(env!("OUT_DIR"), "/mermaid_note.rs"));
include!(concat!(env!("OUT_DIR"), "/mermaid_blocks.rs"));

/// The fonts a diagram's text is drawn with: the platform's own folder, and `sans-serif` (the
/// family the SVG falls back to) named after a face in it.
fn fontdb() -> Arc<resvg::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        if cfg!(target_os = "android") {
            db.load_fonts_dir("/system/fonts");
        } else if cfg!(target_os = "ios") {
            db.load_fonts_dir("/System/Library/Fonts");
        } else {
            db.load_system_fonts();
        }
        let installed = |family: &str| {
            db.faces()
                .any(|face| face.families.iter().any(|(name, _)| name == family))
        };
        if let Some(family) = ["Roboto", "Helvetica Neue", "Helvetica", "Arial"]
            .into_iter()
            .find(|family| installed(family))
        {
            db.set_sans_serif_family(family);
        }
        Arc::new(db)
    })
    .clone()
}

/// The drawing's SVG and its own size, or why the source is shown instead. Runs off the UI thread.
pub(super) fn svg(source: &str, light: bool) -> Result<(Arc<str>, f32, f32), String> {
    if let Some(note) = unsupported_note(source, light) {
        return Err(note.to_string());
    }
    let svg = mermaid_svg(source, light)
        .map_err(|error| format!("This diagram could not be drawn: {error}"))?;
    let (width, height) = svg_natural_size(&svg)
        .ok_or_else(|| "The diagram image could not be displayed.".to_string())?;
    Ok((svg.into(), width, height))
}

/// The SVG painted at `scale` device pixels per SVG unit. Runs off the UI thread.
pub(super) fn raster(svg: &str, scale: f32) -> Option<Arc<RenderImage>> {
    rasterize_svg_scaled(svg, scale)
}

/// The phone has no larger diagram viewer to open.
pub(super) const HAS_VIEWER: bool = false;
