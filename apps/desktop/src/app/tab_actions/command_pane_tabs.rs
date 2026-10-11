//! Command pane tabs: select, wake, sleep, close and focus mode, per tab and per scope.

use gpui::Keystroke;
use gpui::Window;

use crate::app::actions::*;
use crate::app::consts::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn select_command_pane_tab(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        expand_pane: bool,
        window: &Window,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-06-25-12:10:
        Collapsed-strip command-tab selection must match native hidden-open behavior: select the clicked command tab, restore the last pinned/floating mode, and reset height from the current Workspace default only while hidden. Expanded titlebar tab selection stays a pure tab focus change.

        CDXC:SessionSleep 2026-06-25-14:46:
        Native command-tab clicks wake sleeping command sessions immediately only when click-to-wake placeholders are disabled. With the default click-to-wake setting, tab selection stays layout-only and the sleeping body click performs wake.

        CDXC:Notifications 2026-06-25-19:58:
        Direct command-tab activation should also acknowledge an Attention command session like native tab/titlebar focus. Clear only the selected command session's Attention state; Working, Delayed Send, sleeping placeholders, and Agents activity keep their existing semantics.
        */
        let settings_snapshot = shared_settings::shared_sidebar_settings_snapshot();
        let click_to_wake_enabled =
            command_pane_click_to_wake_sleeping_sessions_from_shared_settings(&settings_snapshot);
        let wake_on_tab_selection = command_pane_sleeping_tab_selection_wake_target(
            &self.command_pane,
            group_id,
            session_id,
            click_to_wake_enabled,
        )
        .is_some();
        let selected = if expand_pane {
            self.command_pane.select_session_in_group_for_hidden_open(
                group_id,
                session_id,
                command_pane_content_height(window),
                command_pane_default_height_px_from_shared_settings(&settings_snapshot),
            )
        } else {
            self.command_pane
                .select_session_in_group(group_id, session_id)
        };
        if !selected {
            return false;
        }
        self.command_pane
            .acknowledge_attention_for_session_activation(session_id);
        let woke_sleeping_session =
            wake_on_tab_selection && self.command_pane.set_session_sleeping(session_id, false);
        if woke_sleeping_session {
            let title = self
                .command_pane
                .session(session_id)
                .map(|session| session.title.clone())
                .unwrap_or_else(|| COMMAND_PANE_DEFAULT_SESSION_TITLE.to_string());
            self.start_command_terminal_gxserver_attach_for_slot(
                CommandTerminalBodyMountSlotId {
                    group_id,
                    session_id,
                },
                title,
                None,
                None,
                None,
                cx,
            );
            self.refresh_gpui_command_close_after_done_timer_for_session(session_id, cx);
        }
        self.focus_command_pane(cx);
        self.request_command_terminal_text_focus_handoff(CommandTerminalBodyMountSlotId {
            group_id,
            session_id,
        });
        self.scroll_command_group_active_tab(group_id);
        self.scroll_focused_command_active_tab();
        self.persist_shell_layout_state();
        if woke_sleeping_session {
            self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        }
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        cx.notify();
        true
    }

    pub(crate) fn wake_command_pane_session(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:SessionSleep 2026-06-25-14:27:
        With the default click-to-wake setting, selecting a sleeping command tab only makes that tab active; activating the sleeping command body wakes it. This mirrors native placeholder behavior and prevents right-click tab menus from recreating a command terminal surface early.
        */
        if !self.command_pane.set_session_sleeping(session_id, false) {
            return false;
        }
        let title = self
            .command_pane
            .session(session_id)
            .map(|session| session.title.clone())
            .unwrap_or_else(|| COMMAND_PANE_DEFAULT_SESSION_TITLE.to_string());
        self.start_command_terminal_gxserver_attach_for_slot(
            CommandTerminalBodyMountSlotId {
                group_id,
                session_id,
            },
            title,
            None,
            None,
            None,
            cx,
        );
        self.refresh_gpui_command_close_after_done_timer_for_session(session_id, cx);
        self.command_pane.focus_group(group_id);
        self.focus_command_pane(cx);
        self.request_command_terminal_text_focus_handoff(CommandTerminalBodyMountSlotId {
            group_id,
            session_id,
        });
        self.scroll_command_group_active_tab(group_id);
        self.scroll_focused_command_active_tab();
        self.persist_shell_layout_state();
        self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        cx.notify();
        true
    }

    pub(crate) fn toggle_command_pane_focus_mode_for_tab(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if !self
            .command_pane
            .toggle_focus_mode_for_tab(group_id, session_id)
        {
            return false;
        }
        self.focus_command_pane(cx);
        self.request_command_terminal_text_focus_handoff(CommandTerminalBodyMountSlotId {
            group_id,
            session_id,
        });
        self.scroll_command_group_active_tab(group_id);
        self.scroll_focused_command_active_tab();
        self.persist_shell_layout_state();
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        cx.notify();
        true
    }

    pub(crate) fn wake_focused_sleeping_command_placeholder_from_keystroke(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some((group_id, session_id)) = focused_sleeping_command_placeholder_wake_target(
            self.shell_focus,
            &self.command_pane,
            keystroke,
        ) else {
            return false;
        };
        self.wake_command_pane_session(group_id, session_id, cx)
    }

    pub(crate) fn wake_focused_sleeping_agents_placeholder_from_keystroke(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some((pane_id, session_id)) = focused_sleeping_agents_placeholder_wake_target(
            self.active_mode,
            self.shell_focus,
            &self.agents_workspace,
            keystroke,
        ) else {
            return false;
        };
        self.activate_agents_terminal_placeholder(pane_id, session_id, cx);
        true
    }

    pub(crate) fn sleep_focused_command_pane_session(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some((group_id, session_id)) =
            focused_command_pane_sleep_target(self.shell_focus, &self.command_pane)
        else {
            return false;
        };
        self.sleep_command_pane_tabs_for_scope(
            group_id,
            session_id,
            CommandPaneTabSleepScope::Sleep,
            CommandPaneScopedTabMutationFocusPolicy::FocusCommandPane,
            cx,
        )
    }

    pub(crate) fn wake_focused_command_pane_session(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some((group_id, session_id)) =
            focused_command_pane_wake_target(self.shell_focus, &self.command_pane)
        else {
            return false;
        };
        self.wake_command_pane_session(group_id, session_id, cx)
    }

    pub(crate) fn close_command_pane_tab(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-07-10:
        macOS command-tab close parity is immediate: native closeTerminal
        removes the tab, tears down the surface, and kills the gxserver zmx
        session without a confirm prompt or surface close-request round trip.
        Close mutates the command model first; the render/bounds-driven host
        reconciliation then drops the stale engine record or Ghostty surface
        (CDXC:Terminal 2026-06-23-05:21 cleanup path).
        */
        if !self
            .command_pane
            .close_session_from_direct_tab_close(group_id, session_id)
        {
            return false;
        }
        self.forget_command_gxserver_session_for_closed_tab(session_id, cx);
        self.clear_command_resize_hover_state_if_command_pane_hidden();
        self.clear_gpui_command_delayed_send_timer(session_id);
        self.clear_gpui_command_close_after_done_timer(session_id);
        if self.command_pane.has_sessions() {
            self.focus_command_pane(cx);
        } else {
            self.restore_previous_non_command_focus_or_default(cx);
        }
        self.scroll_command_group_active_tab(group_id);
        self.scroll_focused_command_active_tab();
        self.persist_shell_layout_state();
        self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        self.refresh_sidebar_command_pane_sessions_if_changed(cx);
        cx.notify();
        true
    }

    /// CDXC:CommandPane 2026-09-23 DECISION:
    /// User: "make right middle clicking on the command pane tabs bar in an empty area just close all the terminals there". A middle-click on empty tab-bar space closes every tab that bar shows: one command group's tabs, or on the collapsed strip every Commands pane tab it lists (never the Terminal view's), each through the same close as the tab's own middle-click.
    pub(crate) fn close_all_command_pane_tabs_in_bar(
        &mut self,
        group_id: Option<CommandPaneGroupId>,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let tabs = match group_id {
            Some(group_id) => self
                .command_pane
                .find_leaf(group_id)
                .map(|leaf| {
                    leaf.tab_group
                        .tabs
                        .iter()
                        .map(|tab| (group_id, tab.session_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            None => self.command_pane.panel_flat_tab_ids(),
        };
        let mut closed = false;
        for (group_id, session_id) in tabs {
            closed |= self.close_command_pane_tab(group_id, session_id, cx);
        }
        closed
    }

    pub(crate) fn close_command_pane_tabs_for_scope(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        scope: CommandPaneTabCloseScope,
        focus_policy: CommandPaneScopedTabMutationFocusPolicy,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-06-25-11:20:
        Bulk command-tab closes must reuse the same close ownership as single command tabs. Every resolved sibling tab is removed from the command model immediately (macOS command close parity, no close-request deferral); the target list is resolved before mutation so Close Left/Right/Others cannot drift while tabs are removed.

        CDXC:ContextMenus 2026-06-25-18:38:
        Scoped Close menu rows are lifecycle requests, not native tab-context primary actions. Do not transfer shell focus just because a GPUI popup menu scoped close removed sibling tabs; direct tab close and focused-session close still use the focus-restoring single-tab close path.
        */
        if scope == CommandPaneTabCloseScope::Close {
            return self.close_command_pane_tab(group_id, session_id, cx);
        }

        let session_ids = self
            .command_pane
            .tab_session_ids_for_close_scope(group_id, session_id, scope);
        if session_ids.is_empty() {
            return false;
        }

        let mut model_changed = false;
        for close_session_id in session_ids {
            if self.command_pane.close_session(group_id, close_session_id) {
                self.forget_command_gxserver_session_for_closed_tab(close_session_id, cx);
                self.clear_gpui_command_delayed_send_timer(close_session_id);
                self.clear_gpui_command_close_after_done_timer(close_session_id);
                model_changed = true;
            }
        }

        if model_changed {
            self.clear_command_resize_hover_state_if_command_pane_hidden();
            if focus_policy == CommandPaneScopedTabMutationFocusPolicy::FocusCommandPane {
                if self.command_pane.has_sessions() {
                    self.focus_command_pane(cx);
                } else {
                    self.restore_previous_non_command_focus_or_default(cx);
                }
            }
            self.scroll_command_group_active_tab(group_id);
            self.scroll_focused_command_active_tab();
            self.sync_gpui_keep_awake_automation_from_current_settings(cx);
            self.persist_shell_layout_state();
            self.refresh_sidebar_command_pane_sessions_if_changed(cx);
            cx.notify();
        }
        model_changed
    }

    pub(crate) fn sleep_command_pane_tabs_for_scope(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        scope: CommandPaneTabSleepScope,
        focus_policy: CommandPaneScopedTabMutationFocusPolicy,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:SessionSleep 2026-06-25-14:27:
        Sleeping command tabs is a lifecycle mutation, not a close. Mark the resolved clicked-group command sessions sleeping so tabs and layout remain intact, the body mount-slot list drops sleeping active sessions, and persistence records only safe enum/boolean state without command text, output, paths, process ids, status-file paths, or terminal content.

        CDXC:DelayedSend 2026-06-25-15:46:
        Scoped command-tab Sleep must not cancel Delayed Send or Close After Done. Preserve native's parked-session contract: Delayed Send remains session-owned and submits only if the tab is awake by the deadline, while Close After Done keeps only its armed intent until wake/Done refresh restarts countdown evaluation.

        CDXC:Sessions 2026-06-27-01:37:
        Sleeping a command tab preserves the Close After Done armed flag but clears any active runtime deadline immediately. The three-minute Done watcher must not keep counting down while the tab is sleeping; wake/Done refresh starts a fresh countdown.

        CDXC:ContextMenus 2026-06-25-18:38:
        Scoped Sleep menu rows dispatch through `paneTabSleepRequested` in native without first focusing the clicked terminal. Preserve GPUI shell focus for GPUI popup menu scoped sleep while focused command Sleep keeps command-pane focus ownership.
        */
        let session_ids = self
            .command_pane
            .tab_session_ids_for_sleep_scope(group_id, session_id, scope);
        if session_ids.is_empty() {
            return false;
        }

        let mut model_changed = false;
        for sleep_session_id in session_ids {
            if self
                .command_pane
                .set_session_sleeping(sleep_session_id, true)
            {
                model_changed = true;
            }
        }
        let close_after_done_timer_changed =
            self.prune_gpui_command_close_after_done_timers_for_command_model();

        if model_changed {
            if focus_policy == CommandPaneScopedTabMutationFocusPolicy::FocusCommandPane {
                self.focus_command_pane(cx);
            }
            self.scroll_command_group_active_tab(group_id);
            self.scroll_focused_command_active_tab();
            self.persist_shell_layout_state();
            self.sync_gpui_keep_awake_automation_from_current_settings(cx);
        }
        if model_changed || close_after_done_timer_changed {
            self.refresh_sidebar_command_pane_sessions_if_changed(cx);
            cx.notify();
        }
        model_changed || close_after_done_timer_changed
    }

    pub(crate) fn sleep_command_pane_tabs_for_scope_from_action(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        scope_value: u8,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(scope) = CommandPaneTabSleepScope::from_action_value(scope_value) else {
            return;
        };
        self.sleep_command_pane_tabs_for_scope(
            group_id,
            session_id,
            scope,
            command_pane_tab_context_scoped_lifecycle_focus_policy(),
            cx,
        );
    }

    pub(crate) fn close_command_pane_tabs_for_scope_from_action(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        scope_value: u8,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(scope) = CommandPaneTabCloseScope::from_action_value(scope_value) else {
            return;
        };
        self.close_command_pane_tabs_for_scope(
            group_id,
            session_id,
            scope,
            command_pane_tab_context_scoped_lifecycle_focus_policy(),
            cx,
        );
    }
}
