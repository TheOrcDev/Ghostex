//! The Bots sidebar's Automations row in the browser: the feed is desktop-only, like Automate.

use gpui::{Context, Window};

use crate::GhostexGpuiApp;

impl GhostexGpuiApp {
    pub(crate) fn bot_feed_showing(&self) -> bool {
        false
    }

    pub(crate) fn toggle_bot_feed_view(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_gpui_workspace_action_toast(
            "info",
            "Not available in the browser",
            "The Automations feed needs the Ghostex app.",
            cx,
        );
    }
}
