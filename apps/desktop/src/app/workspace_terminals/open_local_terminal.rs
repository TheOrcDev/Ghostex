//! Focusing, selecting and opening local workspace terminals, with or without keeping the current view.

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn focus_existing_gpui_local_workspace_terminal(
        &mut self,
        key: &GpuiLocalWorkspaceSessionKey,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        self.prune_local_workspace_session_mappings();
        let Some(shell_session_id) = self.local_workspace_session_mappings.get(key).copied() else {
            return false;
        };
        let Some(pane_id) = self.agents_workspace.pane_id_for_session(shell_session_id) else {
            self.local_workspace_session_mappings.remove(key);
            return false;
        };
        if !self.local_workspace_terminal_can_focus_existing(pane_id, shell_session_id) {
            return false;
        }
        // The selection rule (session_pane_placement.rs): the session comes to the focused pane
        // unless it is already on screen in another one.
        let pane_id = self.pull_workspace_session_into_focused_pane(pane_id, shell_session_id);

        /*
        CDXC:FocusRouting 2026-06-26-06:34:
        Focusing an already-mapped local gxserver session reuses the existing GPUI tab only after the session has a live terminal owner, a reusable viewer recipe, or an inserted attach payload for the exact mount slot. Reconciled sidebar placeholders without attach state intentionally fall through to the gxserver attach pipeline so they cannot mount a default shell.
        */
        focus_existing_local_workspace_terminal_tab_model(
            &mut self.agents_workspace,
            &mut self.agents_terminal_runtime_sessions,
            pane_id,
            shell_session_id,
        );
        self.activate_preferred_agents_chat_launch_intent(shell_session_id, cx);
        self.adopt_preferred_chat_view_on_selection(shell_session_id, cx);
        // CDXC:Workarea 2026-09-20 WHY:
        // Selecting a session focuses its pane in the Agents column and leaves the view panel alone;
        // it used to have to choose between the companion and switching the whole workarea.
        self.focus_shell_target(ShellFocusTarget::AgentsPane(pane_id), cx);
        self.set_sidebar_focus_border_handoff_target(shell_session_id);
        self.request_agents_session_text_focus_handoff(
            AgentsTerminalBodyMountSlotId {
                pane_id,
                session_id: shell_session_id,
            },
            cx,
        );
        self.scroll_workspace_pane_active_tab(pane_id);
        /*
        Sidebar-originated focus updates React optimistically before this
        native tab selection runs. Publish the post-selection workspace owners
        so the sidebar replaces that provisional click-history set with the
        exact currently surfaced panes: one focused session plus the selected
        session in every other rendered split.
        */
        self.dispatch_gpui_workspace_tab_session_selected(
            key.project_id.as_str(),
            key.session_id.as_str(),
            false,
            false,
            cx,
        );
        self.persist_shell_layout_state();
        self.update_active_mode_cef_child_visibility(cx);
        cx.notify();
        true
    }

    /// CDXC:Navigation 2026-09-11 WHY:
    /// The keep-view half of the sidebar focus pipeline: the same tab selection, mapping, sidebar publish, and persistence as `focus_existing_gpui_local_workspace_terminal`, without the switch to Agents and without moving keyboard focus into the pane, so a project left on Code, Browser, Kanban, Automate, or Docs comes back exactly there with the session waiting as the active Agents tab.
    /// `local_workspace_latest_focus_key` is deliberately left alone: a surfaced-restore attach that finds it equal to its own key promotes itself to a click, which is the Agents switch this path exists to avoid.
    pub(crate) fn select_local_workspace_terminal_keeping_view(
        &mut self,
        key: &GpuiLocalWorkspaceSessionKey,
        wake_sleeping: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        support_logs::append(
            support_logs::GpuiSupportLog::TerminalFocus,
            "gpui.terminalFocus.keptView",
            serde_json::json!({
                "projectId": key.project_id,
                "sessionId": key.session_id,
            }),
        );
        self.refresh_sidebar_gxserver_bootstrap_if_changed(cx);
        if !wake_sleeping && self.select_existing_local_workspace_terminal_keeping_view(key, cx) {
            self.reconcile_preferred_agents_chat_launch_intents(cx);
            return;
        }
        let mapped_attach_intent = self.local_workspace_attach_intent_for_key(key);
        // The background half of the same one-round-trip wake the ordinary path uses (`wake_sleeping` on the focus message).
        let attach_intent = if wake_sleeping {
            GpuiLocalWorkspaceAttachIntent::Wake
        } else {
            mapped_attach_intent
        };
        let requested_pane_id = self
            .local_workspace_session_mappings
            .get(key)
            .copied()
            .and_then(|shell_session_id| {
                self.agents_workspace.pane_id_for_session(shell_session_id)
            })
            .unwrap_or(self.agents_workspace.focused_pane);
        self.spawn_local_workspace_attach_plan(
            key.clone(),
            attach_intent,
            requested_pane_id,
            false,
            GpuiLocalWorkspaceAttachOrigin::BackgroundSelect,
            cx,
        );
    }

    /// Selects an already-mapped tab in place when something local can render
    /// it (a live terminal owner or a pending attach payload), touching neither
    /// the mode nor keyboard focus. False means the session has no usable tab
    /// and the caller attaches it silently.
    fn select_existing_local_workspace_terminal_keeping_view(
        &mut self,
        key: &GpuiLocalWorkspaceSessionKey,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        self.prune_local_workspace_session_mappings();
        let Some(shell_session_id) = self.local_workspace_session_mappings.get(key).copied() else {
            return false;
        };
        let Some(pane_id) = self.agents_workspace.pane_id_for_session(shell_session_id) else {
            self.local_workspace_session_mappings.remove(key);
            return false;
        };
        if !self.local_workspace_terminal_can_focus_existing(pane_id, shell_session_id) {
            return false;
        }
        let pane_id = self.pull_workspace_session_into_focused_pane(pane_id, shell_session_id);
        focus_existing_local_workspace_terminal_tab_model(
            &mut self.agents_workspace,
            &mut self.agents_terminal_runtime_sessions,
            pane_id,
            shell_session_id,
        );
        self.activate_preferred_agents_chat_launch_intent(shell_session_id, cx);
        self.finish_local_workspace_terminal_background_selection(key, pane_id, cx);
        true
    }

    /// Shared tail of both keep-view paths: reveal the tab in its strip,
    /// publish the selection to the sidebar, persist, and repaint.
    pub(crate) fn finish_local_workspace_terminal_background_selection(
        &mut self,
        key: &GpuiLocalWorkspaceSessionKey,
        pane_id: WorkspacePaneId,
        cx: &mut gpui::Context<Self>,
    ) {
        self.scroll_workspace_pane_active_tab(pane_id);
        self.dispatch_gpui_workspace_tab_session_selected(
            key.project_id.as_str(),
            key.session_id.as_str(),
            false,
            false,
            cx,
        );
        self.persist_shell_layout_state();
        self.update_active_mode_cef_child_visibility(cx);
        cx.notify();
    }

    /// Attach completion for `GpuiLocalWorkspaceAttachOrigin::BackgroundSelect`:
    /// `open_gpui_local_workspace_terminal` minus the Agents switch and the
    /// focus handoff. The mount-slot payload waits until Agents is shown again.
    pub(crate) fn open_gpui_local_workspace_terminal_keeping_view(
        &mut self,
        key: GpuiLocalWorkspaceSessionKey,
        plan: GpuiLocalWorkspaceAttachTerminalPlan,
        requested_pane_id: WorkspacePaneId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if self.select_existing_local_workspace_terminal_keeping_view(&key, cx) {
            return true;
        }
        let result = insert_gpui_local_workspace_attach_terminal(
            &mut self.agents_workspace,
            &mut self.agents_terminal_runtime_sessions,
            &mut self.agents_terminal_launch_payload_source,
            &mut self.local_workspace_session_mappings,
            requested_pane_id,
            false,
            key.clone(),
            plan,
        );
        let (pane_id, session_id) = match result {
            Ok(inserted) => inserted,
            Err(message) => {
                support_logs::append(
                    support_logs::GpuiSupportLog::TerminalFocus,
                    "gpui.terminalFocus.keptViewAttachFailed",
                    serde_json::json!({
                        "projectId": key.project_id,
                        "reason": message,
                        "sessionId": key.session_id,
                    }),
                );
                return false;
            }
        };
        self.activate_preferred_agents_chat_launch_intent(session_id, cx);
        self.finish_local_workspace_terminal_background_selection(&key, pane_id, cx);
        true
    }

    pub(crate) fn open_gpui_local_workspace_terminal(
        &mut self,
        key: GpuiLocalWorkspaceSessionKey,
        plan: GpuiLocalWorkspaceAttachTerminalPlan,
        requested_pane_id: WorkspacePaneId,
        force_requested_pane_placement: bool,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if self.focus_existing_gpui_local_workspace_terminal(&key, cx) {
            return true;
        }
        let focused_project_id = key.project_id.clone();
        let focused_session_id = key.session_id.clone();
        /*
        CDXC:FocusRouting 2026-06-26-06:08:
        Local sidebar clicks create or focus Agents workspace tabs like macOS session attach: existing mapped tabs are reused, while new local gxserver attaches start as selected Running mount slots and receive the daemon-built attach command through a one-shot process-local launch payload. Shell persistence keeps layout/lifecycle metadata and must not store commands, paths, daemon bodies, renderer labels, titles from CEF, tokens, stdout/stderr, or terminal content.

        CDXC:FocusRouting 2026-06-26-06:18:
        MacOS decides the workspace tab group at sidebar activation time, then lets async wake/attach complete against that focus intent. GPUI must pass the captured Agents pane through attach completion so focusing another pane while gxserver prepares metadata cannot move the restored session into the wrong tab group.
        */
        let result = insert_gpui_local_workspace_attach_terminal(
            &mut self.agents_workspace,
            &mut self.agents_terminal_runtime_sessions,
            &mut self.agents_terminal_launch_payload_source,
            &mut self.local_workspace_session_mappings,
            requested_pane_id,
            force_requested_pane_placement,
            key,
            plan,
        );
        let (pane_id, session_id) = match result {
            Ok(inserted) => inserted,
            Err(message) => {
                self.cancel_sidebar_focus_border_handoff();
                self.dispatch_gpui_app_modal_toast(
                    "warning",
                    "Session attach unavailable",
                    message,
                    cx,
                );
                return false;
            }
        };
        self.activate_preferred_agents_chat_launch_intent(session_id, cx);
        self.adopt_preferred_chat_view_on_selection(session_id, cx);
        {
            self.focus_shell_target(ShellFocusTarget::AgentsPane(pane_id), cx);
            self.set_sidebar_focus_border_handoff_target(session_id);
            self.request_agents_session_text_focus_handoff(
                AgentsTerminalBodyMountSlotId {
                    pane_id,
                    session_id,
                },
                cx,
            );
        }
        self.scroll_workspace_pane_active_tab(pane_id);
        self.dispatch_gpui_workspace_tab_session_selected(
            focused_project_id.as_str(),
            focused_session_id.as_str(),
            false,
            false,
            cx,
        );
        self.persist_shell_layout_state();
        self.update_active_mode_cef_child_visibility(cx);
        self.reveal_floating_sessions(cx);
        cx.notify();
        true
    }

    pub(crate) fn open_gpui_local_workspace_terminal_in_new_leaf(
        &mut self,
        key: GpuiLocalWorkspaceSessionKey,
        plan: GpuiLocalWorkspaceAttachTerminalPlan,
        requested_pane_id: WorkspacePaneId,
        placement: AgentsWorkspaceNewTerminalPlacement,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-07-24:
        Split-right/split-below/bottom-row quick creation opens the freshly
        created gxserver session in a new workspace leaf instead of a tab in the
        requested pane. The session is otherwise identical to a sidebar attach:
        Running presentation, one-shot mount-slot attach payload, and
        local-workspace mappings so the sidebar lists it and project switches
        keep it.
        */
        if self.focus_existing_gpui_local_workspace_terminal(&key, cx) {
            return true;
        }
        let focused_project_id = key.project_id.clone();
        let focused_session_id = key.session_id.clone();
        let result = insert_gpui_local_workspace_attach_terminal_in_new_leaf(
            &mut self.agents_workspace,
            &mut self.agents_terminal_runtime_sessions,
            &mut self.agents_terminal_launch_payload_source,
            &mut self.local_workspace_session_mappings,
            requested_pane_id,
            placement,
            key,
            plan,
        );
        let (pane_id, session_id) = match result {
            Ok(inserted) => inserted,
            Err(message) => {
                self.dispatch_gpui_app_modal_toast(
                    "warning",
                    "Session attach unavailable",
                    message,
                    cx,
                );
                return false;
            }
        };
        self.activate_preferred_agents_chat_launch_intent(session_id, cx);
        self.change_active_mode_with_pane_state(TitlebarMode::Agents, cx);
        self.focus_shell_target(ShellFocusTarget::AgentsPane(pane_id), cx);
        self.set_sidebar_focus_border_handoff_target(session_id);
        self.request_agents_session_text_focus_handoff(
            AgentsTerminalBodyMountSlotId {
                pane_id,
                session_id,
            },
            cx,
        );
        self.workspace_drop_feedback = None;
        self.scroll_workspace_pane_active_tab(pane_id);
        self.dispatch_gpui_workspace_tab_session_selected(
            focused_project_id.as_str(),
            focused_session_id.as_str(),
            false,
            false,
            cx,
        );
        self.persist_shell_layout_state();
        self.update_active_mode_cef_child_visibility(cx);
        self.reveal_floating_sessions(cx);
        cx.notify();
        true
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn compensate_unmaterialized_created_workspace_terminal(
        &mut self,
        key: &GpuiLocalWorkspaceSessionKey,
    ) {
        /*
        This runs inside the serialized GPUI completion update. Re-check the
        canonical mapping immediately before exact close/remove compensation so
        a session already materialized by presentation reconciliation is never
        deleted. The synchronous local cleanup is reserved for the rare case
        where the originally captured pane was deleted while creation ran.
        */
        if self.local_workspace_session_mappings.contains_key(key) {
            return;
        }
        let _ = gpui_close_command_terminal_gxserver_session(key);
    }
}
