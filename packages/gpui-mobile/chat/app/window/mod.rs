//! The two window helpers the chat's child windows call, plus the one colour mix it takes from the
//! desktop's native modal kit.
#[allow(dead_code)]
pub(crate) mod popup_frame;

#[allow(dead_code, unused_imports)]
pub(crate) mod native_modal_kit {
    use gpui::Rgba;
    include!(concat!(env!("OUT_DIR"), "/native_modal_kit.rs"));
}

/// AppKit child-window attachment on the desktop. The phone's chat opens its child windows (menus,
/// the image viewer, the table preview) as the platform's own windows, which it stacks itself.
pub(crate) fn attach_gpui_app_modal_window_to_main_window(
    _window: &mut gpui::Window,
    _main_window_native_view: *mut std::ffi::c_void,
) {
}
