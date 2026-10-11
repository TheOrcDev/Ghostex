//! iOS platform implementation for GPUI.
//!
//! iOS uses UIKit instead of AppKit, so the platform implementation differs
//! significantly from macOS despite sharing many underlying technologies:
//! - Grand Central Dispatch (GCD) for threading
//! - CoreText for text rendering
//! - Metal for GPU rendering
//! - CoreFoundation for many utilities

pub(crate) mod cg_types;
mod dispatcher;
mod display;
mod document_picker;
mod events;
pub mod ffi;
// Ghostex patch: see panic_guard.rs.
pub mod panic_guard;
mod platform;
pub mod platform_view;
mod text_input;
mod text_system;
// Ghostex patch: see touch_samples.rs.
mod touch_samples;
pub mod util;
mod window;

pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub use platform::*;
pub(crate) use text_system::*;
pub use window::set_status_bar_style;
// Ghostex patch: the iOS twin of `android::window::set_fling_guard_enabled`.
pub use touch_samples::set_fling_guard_enabled;
pub(crate) use window::*;

/// Returns the platform implementation for iOS.
pub fn current_platform(_headless: bool) -> std::rc::Rc<dyn gpui::Platform> {
    std::rc::Rc::new(IosPlatform::new())
}
