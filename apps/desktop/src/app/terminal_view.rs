//! The Terminal view: the Commands pane's second tree (`CommandPaneDock::View`) shown as a tab of
//! the view panel. The model rules live in `model/command_pane_docks.rs`; this is the app side:
//! when the view is on screen, how it gets its first tab, and what happens when it loses its last.

use gpui::{AnyWindowHandle, Window};

use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn terminal_view_on_screen(&self) -> bool {
        self.open_view_mode() == Some(TitlebarMode::Terminal)
    }

    /// The model reads dock visibility instead of the app's mode, so every mode change and project
    /// swap writes it back here before any terminal reconcile runs.
    pub(crate) fn sync_command_pane_view_dock_visibility(&mut self) {
        self.command_pane.view_dock_visible = self.terminal_view_on_screen();
    }

    /// Opening the Terminal view is F12 for its own tree: keep the tab it had, or create its first
    /// `Command Terminal`, and point `focused_group` at it so the mode's default focus lands there.
    /// Returns whether the view has a tab to show.
    pub(crate) fn seed_terminal_view_for_open(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        self.sync_command_pane_view_dock_visibility();
        if !self.terminal_view_on_screen() {
            return false;
        }
        let Some((group_id, session_id, created)) =
            self.command_pane.ensure_view_session_for_open()
        else {
            return false;
        };
        if created {
            self.start_command_terminal_gxserver_attach_for_slot(
                CommandTerminalBodyMountSlotId {
                    group_id,
                    session_id,
                },
                COMMAND_PANE_DEFAULT_SESSION_TITLE.to_string(),
                None,
                None,
                None,
                cx,
            );
        }
        self.command_pane
            .acknowledge_attention_for_live_focused_group_activation();
        self.scroll_command_group_active_tab(group_id);
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        true
    }

    /// Runs `f` with the main window on the next turn, for work that needs a `Window` from a place
    /// that has none (the command model's post-mutation hooks).
    pub(crate) fn defer_in_main_window(
        &self,
        cx: &mut gpui::Context<Self>,
        f: impl FnOnce(&mut Self, &mut Window, &mut gpui::Context<Self>) + 'static,
    ) {
        let Some(handle): Option<AnyWindowHandle> = self.main_window_handle else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let _ = handle.update(cx, |_, window, cx| {
                let _ = this.update(cx, |app, cx| f(app, window, cx));
            });
        })
        .detach();
    }

    /// The Terminal view goes the way of an emptied Commands pane: when its last tab closes, the
    /// view tab closes with it. Reopening the view creates a fresh tab.
    pub(crate) fn close_terminal_view_if_empty(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.command_pane.has_view_sessions()
            || !self.open_views.contains(&TitlebarMode::Terminal)
        {
            return;
        }
        self.close_view_tab(TitlebarMode::Terminal, window, cx);
    }

    /// Called from the command model's shared post-mutation hook. Every close route (direct close,
    /// scoped close, process exit, Action end, a drag that moves the last tab out) passes through
    /// it, so no route has to remember the Terminal view on its own.
    pub(crate) fn schedule_close_terminal_view_if_empty(&mut self, cx: &mut gpui::Context<Self>) {
        if self.command_pane.has_view_sessions()
            || !self.open_views.contains(&TitlebarMode::Terminal)
        {
            return;
        }
        self.defer_in_main_window(cx, |app, window, cx| {
            app.close_terminal_view_if_empty(window, cx);
        });
    }

    /// Bring the dock a command group lives in on screen: expand the Commands pane, or open the
    /// Terminal view tab. Used where a tab is selected from outside its dock (an Action rerun, a
    /// Delayed Send from a tab menu).
    pub(crate) fn reveal_command_group_dock(
        &mut self,
        group_id: CommandPaneGroupId,
        cx: &mut gpui::Context<Self>,
    ) {
        match self.command_pane.dock_for_group(group_id) {
            Some(CommandPaneDock::Panel) => self.command_pane.expand(),
            Some(CommandPaneDock::View) => {
                if !self.terminal_view_on_screen() {
                    self.defer_in_main_window(cx, |app, window, cx| {
                        app.open_view_tab(TitlebarMode::Terminal, window, cx);
                    });
                }
            }
            None => {}
        }
    }
}
