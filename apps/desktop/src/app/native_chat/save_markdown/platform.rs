pub(in crate::app::native_chat) fn prepare(window: &mut gpui::Window) {
    #[cfg(target_os = "macos")]
    {
        use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
        unsafe extern "C" {
            fn GhostexGpuiPrepareChatDialogWindow(view: *mut std::ffi::c_void);
        }
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::AppKit(handle) = handle.as_raw()
        {
            unsafe { GhostexGpuiPrepareChatDialogWindow(handle.ns_view.as_ptr()) };
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}
