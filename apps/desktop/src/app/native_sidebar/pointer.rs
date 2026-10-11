use gpui::{Bounds, Pixels, Window};
use std::ffi::c_void;

unsafe extern "C" {
    fn GhostexGpuiNativeSidebarSetTrackingBounds(
        view: *mut c_void,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    );
}

pub(super) fn track_bounds(bounds: Bounds<Pixels>, window: &Window) {
    if let Ok(view) = crate::cef_parent_native_view(window) {
        unsafe {
            GhostexGpuiNativeSidebarSetTrackingBounds(
                view,
                f32::from(bounds.left()) as f64,
                f32::from(bounds.top()) as f64,
                f32::from(bounds.size.width) as f64,
                f32::from(bounds.size.height) as f64,
            );
        }
    }
}
