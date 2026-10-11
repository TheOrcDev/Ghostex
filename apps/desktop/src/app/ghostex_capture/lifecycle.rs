//! Turning Ghostex Capture on and off with its setting, the hotkey listener, and what each action
//! does.

use futures::StreamExt as _;
use gpui::Context;

use super::hotkeys;
use super::model::CaptureAction;
use super::persistence;
use super::platform;
use crate::GhostexGpuiApp;
use crate::shared_settings;

impl GhostexGpuiApp {
    /// Called once the app entity exists.
    pub(crate) fn initialize_ghostex_capture(&mut self, cx: &mut Context<Self>) {
        self.ghostex_capture.saved = persistence::load();
        let snapshot = shared_settings::shared_sidebar_settings_snapshot();
        self.ghostex_capture_settings_changed(&snapshot, cx);
    }

    /// Follows the Ghostex Capture setting after a Settings save.
    pub(crate) fn ghostex_capture_settings_changed(
        &mut self,
        settings: &shared_settings::SharedSidebarSettingsSnapshot,
        cx: &mut Context<Self>,
    ) {
        let enabled = settings.ghostex_capture_enabled() && platform::supported();
        let was_enabled = self.ghostex_capture.enabled;
        self.ghostex_capture.enabled = enabled;
        if enabled && !was_enabled && self.ghostex_capture.saved.hidden {
            // Turning the setting on is the way back from "Hide button".
            self.ghostex_capture.saved.hidden = false;
            persistence::save(&self.ghostex_capture.saved);
        }
        if enabled && !self.ghostex_capture.hotkeys_registered {
            self.register_ghostex_capture_hotkeys(cx);
        } else if !enabled && self.ghostex_capture.hotkeys_registered {
            hotkeys::unregister();
            self.ghostex_capture.hotkeys_registered = false;
        }
        self.sync_ghostex_capture(cx);
    }

    /// Shows or hides the button to match the setting and "Hide button", and keeps it in place.
    pub(crate) fn sync_ghostex_capture(&mut self, cx: &mut Context<Self>) {
        let wanted = self.ghostex_capture.enabled && !self.ghostex_capture.saved.hidden;
        if !wanted {
            self.close_ghostex_capture_panel(cx);
            self.close_ghostex_capture_icon(cx);
            return;
        }
        if self.ghostex_capture.icon.is_none() {
            self.open_ghostex_capture_icon(cx);
        } else {
            self.sync_ghostex_capture_icon(cx);
        }
    }

    fn register_ghostex_capture_hotkeys(&mut self, cx: &mut Context<Self>) {
        let Some(mut presses) = hotkeys::register() else {
            return;
        };
        self.ghostex_capture.hotkeys_registered = true;
        platform::start_frontmost_tracking();
        cx.spawn(async move |this, cx| {
            while let Some(action) = presses.next().await {
                if this
                    .update(cx, |app, cx| {
                        // A hotkey fires while the user is in another app: remember it before
                        // anything of ours takes the keyboard.
                        app.ghostex_capture.frontmost = platform::frontmost_app();
                        app.run_ghostex_capture_action(action, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn run_ghostex_capture_action(
        &mut self,
        action: CaptureAction,
        cx: &mut Context<Self>,
    ) {
        if !self.ghostex_capture.enabled {
            return;
        }
        match action {
            CaptureAction::TogglePanel => {
                if self.ghostex_capture.saved.hidden {
                    self.ghostex_capture.saved.hidden = false;
                    persistence::save(&self.ghostex_capture.saved);
                    self.sync_ghostex_capture(cx);
                }
                if self.ghostex_capture.panel.is_some() {
                    self.close_ghostex_capture_panel(cx);
                } else if self.ghostex_capture.icon.is_some() {
                    self.open_ghostex_capture_panel(cx);
                } else {
                    // The button is still opening (it was hidden); open the panel once it is up.
                    let app = cx.weak_entity();
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(120))
                            .await;
                        let _ = app.update(cx, |app, cx| app.open_ghostex_capture_panel(cx));
                    })
                    .detach();
                }
            }
            CaptureAction::Area
            | CaptureAction::CurrentApp
            | CaptureAction::FullScreen
            | CaptureAction::Prompt
            | CaptureAction::ContinuePrompt => {
                self.close_ghostex_capture_panel(cx);
                self.start_ghostex_capture(action, cx);
            }
        }
    }

    /// "Hide button": gone until the setting is turned on again or Cmd/Alt+Ctrl+Shift+S.
    pub(crate) fn hide_ghostex_capture(&mut self, cx: &mut Context<Self>) {
        self.ghostex_capture.saved.hidden = true;
        persistence::save(&self.ghostex_capture.saved);
        self.sync_ghostex_capture(cx);
    }

    pub(crate) fn shut_down_ghostex_capture(&mut self) {
        if self.ghostex_capture.hotkeys_registered {
            hotkeys::unregister();
            self.ghostex_capture.hotkeys_registered = false;
        }
    }

    /// The window that ran Ghostex Capture is closing while another stays open: its button and
    /// panel close and its hotkeys go, for the window taking over to set up again
    /// (app/workspace_windows/).
    pub(crate) fn release_ghostex_capture(&mut self, cx: &mut Context<Self>) {
        self.ghostex_capture.enabled = false;
        self.sync_ghostex_capture(cx);
        self.shut_down_ghostex_capture();
    }
}
