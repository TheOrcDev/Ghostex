//! The symlinked entries under this folder ARE the desktop app's source files, compiled here
//! unchanged (symlinks rather than `#[path]`, for the reason `apps/gpui-web/src/app/mod.rs` gives).
//! Real files are the phone's answers to the desktop's native-only halves.
#[allow(dead_code, unused_imports)]
pub(crate) mod consts {
    use std::sync::atomic::AtomicU64;
    include!(concat!(env!("OUT_DIR"), "/consts.rs"));
}
/// Every chat host file compiles; the few entry points only the desktop app calls stay unused here.
#[allow(dead_code)]
pub(crate) mod gx_chat;
pub(crate) mod helpers;
#[allow(dead_code, unused_imports)]
pub(crate) mod hotkeys {
    use crate::*;
    include!(concat!(env!("OUT_DIR"), "/hotkeys.rs"));

    /// The desktop's `gpui_configured_hotkey_label`, which names a shortcut's chord in the chat's
    /// tooltips and on the scroll-to-bottom pill.
    ///
    /// CDXC:Mobile 2026-09-27 WHY: a phone has no keyboard to press them on, so the transcript advertises none; with the desktop's function the pill read "Scroll to bottom (Ctrl+Shift+\u{2193})".
    pub(crate) fn gpui_configured_hotkey_label(_action_id: &str) -> Option<String> {
        None
    }
}
pub(crate) mod mobile_app;
pub(crate) mod model;
#[allow(dead_code)]
pub(crate) mod native_chat;
pub(crate) mod window;
