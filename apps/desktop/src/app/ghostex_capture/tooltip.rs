//! The tooltip every Ghostex Capture window shows.

use gpui::{AnyView, App, Styled, Window, rgb};
use gpui_component::text::Text;
use gpui_component::tooltip::Tooltip;

/// A solid bubble in Ghostex Capture's own colours.
///
/// CDXC:GhostexCapture 2026-09-30 DECISION:
/// User: "make the continue your draft and other tooltips in the whole floating capture feature not
/// transparent please even if transparency enabled in the app". The theme's popover colour is
/// see-through under app transparency, so these bubbles set their own opaque fill.
pub(crate) fn solid_tooltip(label: impl Into<Text>, window: &mut Window, cx: &mut App) -> AnyView {
    Tooltip::new(label)
        .bg(rgb(0x1f1f1f))
        .border_color(rgb(0x3a3a3a))
        .text_color(rgb(0xe8e8e8))
        .build(window, cx)
}
