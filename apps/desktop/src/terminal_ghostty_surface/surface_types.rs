use std::{
    ffi::c_char,
    ptr::{self},
};

use gpui::{Bounds, Pixels};

use crate::ghostty_kit::ffi;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GhosttySurfacePixelSize {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl GhosttySurfacePixelSize {
    pub(crate) fn from_gpui_bounds(
        bounds: Bounds<Pixels>,
        scale_factor: f64,
    ) -> Result<Self, GhosttySurfaceRuntimeError> {
        let scale_factor = GhosttySurfaceScaleFactor::new(scale_factor)?;
        Ok(Self {
            width: scaled_pixel_dimension(
                GhosttySurfaceBoundsField::Width,
                f64::from(bounds.size.width.as_f32()),
                scale_factor,
            )?,
            height: scaled_pixel_dimension(
                GhosttySurfaceBoundsField::Height,
                f64::from(bounds.size.height.as_f32()),
                scale_factor,
            )?,
        })
    }
}

fn scaled_pixel_dimension(
    field: GhosttySurfaceBoundsField,
    value: f64,
    scale_factor: GhosttySurfaceScaleFactor,
) -> Result<u32, GhosttySurfaceRuntimeError> {
    if !value.is_finite() || value < 0.0 {
        return Err(GhosttySurfaceRuntimeError::InvalidBounds { field, value });
    }

    let scaled = (value * scale_factor.get()).floor().max(1.0);
    if !scaled.is_finite() || scaled > f64::from(u32::MAX) {
        return Err(GhosttySurfaceRuntimeError::InvalidBounds { field, value });
    }

    Ok(scaled as u32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // no constructor: key-binding probing is done by ghostty itself now
pub(crate) struct GhosttySurfaceKeyBindingStatus {
    binding: bool,
    flags: ffi::ghostty_binding_flags_e,
}

impl GhosttySurfaceKeyBindingStatus {
    #[allow(dead_code)] // no live caller: key-binding probing is only reachable from the superseded native key path
    pub(crate) fn from_ffi_result(binding: bool, flags: ffi::ghostty_binding_flags_e) -> Self {
        Self {
            binding,
            flags: if binding { flags } else { 0 },
        }
    }

    #[allow(dead_code)] // no live caller: key-binding probing is only reachable from the superseded native key path
    pub(crate) fn binding(self) -> bool {
        self.binding
    }

    #[allow(dead_code)] // no live caller: key-binding probing is only reachable from the superseded native key path
    pub(crate) fn flags(self) -> ffi::ghostty_binding_flags_e {
        self.flags
    }
}

static GHOSTTY_SURFACE_EMPTY_TEXT_SENTINEL: [u8; 1] = [0];

pub(crate) fn ghostty_surface_text_ptr(bytes: &[u8]) -> *const c_char {
    if bytes.is_empty() {
        GHOSTTY_SURFACE_EMPTY_TEXT_SENTINEL.as_ptr() as *const c_char
    } else {
        bytes.as_ptr() as *const c_char
    }
}

pub(crate) fn ghostty_surface_preedit_ptr(bytes: &[u8]) -> *const c_char {
    if bytes.is_empty() {
        ptr::null()
    } else {
        bytes.as_ptr() as *const c_char
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GhosttySurfaceImePoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GhosttySurfaceMetadataSnapshot {
    process_exited: bool,
    foreground_process_id_present: bool,
    tty_name_present: bool,
}

impl GhosttySurfaceMetadataSnapshot {
    pub(crate) fn from_redacted_presence(
        process_exited: bool,
        foreground_process_id_present: bool,
        tty_name_present: bool,
    ) -> Self {
        Self {
            process_exited,
            foreground_process_id_present,
            tty_name_present,
        }
    }

    pub(crate) fn process_exited(self) -> bool {
        self.process_exited
    }

    #[allow(dead_code)] // no live caller: only the superseded startup-host reconcile read surface metadata presence
    pub(crate) fn foreground_process_id_present(self) -> bool {
        self.foreground_process_id_present
    }

    #[allow(dead_code)] // no live caller: only the superseded startup-host reconcile read surface metadata presence
    pub(crate) fn tty_name_present(self) -> bool {
        self.tty_name_present
    }

    pub(crate) fn indicates_ready_metadata(self) -> bool {
        !self.process_exited && self.foreground_process_id_present && self.tty_name_present
    }
}

#[allow(dead_code)] // no live caller: only reachable from the superseded startup-host reconcile
pub(crate) fn ghostty_surface_metadata_snapshot(
    functions: GhosttyKitFunctionTable,
    surface: ffi::ghostty_surface_t,
) -> GhosttySurfaceMetadataSnapshot {
    let process_exited = unsafe { (functions.surface_process_exited)(surface) };
    let foreground_process_id_present = unsafe { (functions.surface_foreground_pid)(surface) } != 0;
    let tty_name = unsafe { (functions.surface_tty_name)(surface) };
    let tty_name_present = !tty_name.ptr.is_null() && tty_name.len > 0;
    unsafe {
        (functions.string_free)(tty_name);
    }

    GhosttySurfaceMetadataSnapshot::from_redacted_presence(
        process_exited,
        foreground_process_id_present,
        tty_name_present,
    )
}
