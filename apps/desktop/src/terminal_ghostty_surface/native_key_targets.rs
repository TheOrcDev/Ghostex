use std::{
    ffi::{c_char, c_void},
    ptr::NonNull,
    sync::{Mutex, OnceLock},
};

use crate::{TerminalSurfaceMountSlotKey, ghostty_kit::ffi};

use super::*;
#[cfg(target_os = "macos")]
use crate::terminal_native_view::RealTerminalNativeViewHandle;

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct GhosttyNativeKeyTarget {
    surface: usize,
    functions: GhosttyKitFunctionTable,
    mount_slot_sort_key: (u8, u64, u64),
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
pub(crate) struct GhosttyNativeKeyTargetDiagnostic {
    pub(crate) surface_kind: u8,
    pub(crate) container_id: u64,
    pub(crate) session_id: u64,
}

#[cfg(target_os = "macos")]
fn ghostty_native_key_targets()
-> &'static Mutex<std::collections::HashMap<usize, GhosttyNativeKeyTarget>> {
    static TARGETS: OnceLock<Mutex<std::collections::HashMap<usize, GhosttyNativeKeyTarget>>> =
        OnceLock::new();
    TARGETS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[cfg(target_os = "macos")]
pub(crate) fn register_native_key_target<SlotId>(
    native_view: RealTerminalNativeViewHandle,
    surface: &GhosttySurfaceOwner<SlotId>,
) where
    SlotId: TerminalSurfaceMountSlotKey,
{
    let target = GhosttyNativeKeyTarget {
        surface: surface.as_raw() as usize,
        functions: surface.functions,
        mount_slot_sort_key: surface.mount_slot_id().terminal_surface_sort_key(),
    };
    if let Ok(mut targets) = ghostty_native_key_targets().lock() {
        targets.insert(native_view.as_ptr() as usize, target);
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn unregister_native_key_target(native_view: RealTerminalNativeViewHandle) {
    if let Ok(mut targets) = ghostty_native_key_targets().lock() {
        targets.remove(&(native_view.as_ptr() as usize));
    }
}

#[cfg(target_os = "macos")]
fn native_key_target_for_view(native_view: *mut c_void) -> Option<GhosttyNativeKeyTarget> {
    let native_view = NonNull::new(native_view)?;
    ghostty_native_key_targets()
        .lock()
        .ok()
        .and_then(|targets| targets.get(&(native_view.as_ptr() as usize)).copied())
}

#[cfg(target_os = "macos")]
pub(crate) fn native_key_target_diagnostic_for_view(
    native_view: *mut c_void,
) -> Option<GhosttyNativeKeyTargetDiagnostic> {
    let target = native_key_target_for_view(native_view)?;
    let (surface_kind, container_id, session_id) = target.mount_slot_sort_key;
    Some(GhosttyNativeKeyTargetDiagnostic {
        surface_kind,
        container_id,
        session_id,
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn native_key_translation_mods_for_view(
    native_view: *mut c_void,
    mods: ffi::ghostty_input_mods_e,
) -> ffi::ghostty_input_mods_e {
    let Some(target) = native_key_target_for_view(native_view) else {
        return mods;
    };
    unsafe { (target.functions.surface_key_translation_mods)(target.surface as *mut c_void, mods) }
}

#[cfg(target_os = "macos")]
pub(crate) fn send_native_key_event_for_view(
    native_view: *mut c_void,
    event: ffi::ghostty_input_key_s,
) -> bool {
    let Some(target) = native_key_target_for_view(native_view) else {
        return false;
    };
    unsafe { (target.functions.surface_key)(target.surface as *mut c_void, event) }
}

#[cfg(target_os = "macos")]
pub(crate) fn native_key_event_is_binding_for_view(
    native_view: *mut c_void,
    event: ffi::ghostty_input_key_s,
) -> bool {
    let Some(target) = native_key_target_for_view(native_view) else {
        return false;
    };
    let mut flags = 0;
    unsafe {
        (target.functions.surface_key_is_binding)(target.surface as *mut c_void, event, &mut flags)
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn send_native_dropped_text_for_view(native_view: *mut c_void, bytes: &[u8]) -> bool {
    send_native_surface_text_for_view(native_view, bytes)
}

#[cfg(target_os = "macos")]
pub(crate) fn send_native_prompt_editor_shortcut_for_view(native_view: *mut c_void) -> bool {
    send_native_surface_text_for_view(native_view, b"\x07")
}

#[cfg(target_os = "macos")]
fn send_native_surface_text_for_view(native_view: *mut c_void, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let Some(target) = native_key_target_for_view(native_view) else {
        return false;
    };
    unsafe {
        (target.functions.surface_text)(
            target.surface as *mut c_void,
            bytes.as_ptr() as *const c_char,
            bytes.len(),
        );
    }
    true
}

#[cfg(target_os = "macos")]
pub(crate) fn set_native_preedit_text_for_view(native_view: *mut c_void, bytes: &[u8]) -> bool {
    let Some(target) = native_key_target_for_view(native_view) else {
        return false;
    };
    unsafe {
        (target.functions.surface_preedit)(
            target.surface as *mut c_void,
            ghostty_surface_preedit_ptr(bytes),
            bytes.len(),
        );
    }
    true
}

#[cfg(target_os = "macos")]
pub(crate) fn native_ime_point_for_view(
    native_view: *mut c_void,
) -> Option<GhosttySurfaceImePoint> {
    let target = native_key_target_for_view(native_view)?;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut width = 0.0;
    let mut height = 0.0;
    unsafe {
        (target.functions.surface_ime_point)(
            target.surface as *mut c_void,
            &mut x,
            &mut y,
            &mut width,
            &mut height,
        );
    }
    Some(GhosttySurfaceImePoint {
        x,
        y,
        width,
        height,
    })
}
