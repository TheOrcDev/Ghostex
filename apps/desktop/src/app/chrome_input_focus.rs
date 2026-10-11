use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    /// CDXC:FocusRouting 2026-09-16 WHY:
    /// gpui makes its content NSView first responder once at window creation and never reclaims it on a click, so clicking a GPUI text input (browser find bar, address bar, terminal search) while a Chromium child view owns the responder only moves GPUI focus and the keys keep flowing into the page.
    /// Cmd+F and the address hotkey reclaim the root before focusing the input; the input's own focus edge is the one place every route (click, hotkey, programmatic) passes through, so the reclaim lives here.
    /// Reclaim only from Chromium work surfaces: a sidebar rename or a titlebar popup that holds the responder while the window re-activates re-fires this edge and must keep the keyboard.
    pub(crate) fn reclaim_gpui_root_for_chrome_input_focus(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if !matches!(
                self.first_responder_target,
                FirstResponderTarget::CefSurface(
                    FirstResponderCefSurface::BrowserTab(_)
                        | FirstResponderCefSurface::ProjectWorkarea(_)
                )
            ) {
                return;
            }
            self.begin_programmatic_focus();
            cef::focus_gpui_root_view(self.parent_ns_view);
            self.end_programmatic_focus();
        }
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            if cef::gpui_root_view_has_native_focus(self.parent_ns_view) {
                return;
            }
            cef::focus_gpui_root_view(self.parent_ns_view);
        }
    }

    /// CDXC:FocusRouting 2026-09-17 WHY:
    /// The native GPUI chat composer replaced the chat CEF page, so no responder transition marks the pane any more: a click on it after a click in a Chromium or Ghostty child view left that child view as AppKit first responder and the keys kept flowing there.
    /// Every keyboard handoff into the native composer therefore yanks the responder onto the GPUI root first, unconditionally, exactly like the composited terminal handoff.
    pub(crate) fn reclaim_gpui_root_for_native_chat_composer(&mut self, window: &gpui::Window) {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            let focus_root = cef_parent_native_view(window).unwrap_or(self.parent_ns_view);
            #[cfg(target_os = "macos")]
            self.begin_programmatic_focus();
            cef::focus_gpui_root_view(focus_root);
            #[cfg(target_os = "macos")]
            self.end_programmatic_focus();
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        let _ = window;
    }
}
