use crate::app::ffi::GPUI_FIRST_RESPONDER_CALLBACK_TARGETS;

#[unsafe(no_mangle)]
pub extern "C" fn GhostexGpuiNavigateHistoryFromNativeView(
    root_view: *mut std::ffi::c_void,
    back: bool,
) -> std::ffi::c_int {
    let target = GPUI_FIRST_RESPONDER_CALLBACK_TARGETS
        .with(|targets| targets.borrow().get(&(root_view as usize)).cloned());
    let Some(target) = target else {
        return 0;
    };
    let mut async_app = target.async_app;
    let foreground = async_app.foreground_executor().clone();
    // Native callbacks can arrive while GPUI is dispatching an event. Enter
    // the app on the next executor turn, as the native keyboard router does.
    foreground
        .spawn(async move {
            let _ = target.app.update_in(&mut async_app, |this, _, cx| {
                if this.app_modal_window.is_none() {
                    this.navigate_history_from_input(back, cx);
                }
            });
        })
        .detach();
    1
}
