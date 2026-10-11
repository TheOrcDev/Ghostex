//! Focusing an agents pane, selecting an agents tab, attention acknowledgement and terminal placeholders.

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn focus_agents_pane(
        &mut self,
        pane_id: WorkspacePaneId,
        cx: &mut gpui::Context<Self>,
    ) {
        self.agents_workspace.focus_pane(pane_id);
        let focused_pane = self.agents_workspace.focused_pane;
        self.focus_shell_target(ShellFocusTarget::AgentsPane(focused_pane), cx);
        support_logs::append(
            support_logs::GpuiSupportLog::TerminalFocus,
            "gpui.terminalFocus.agentsPaneFocusRequested",
            serde_json::json!({
                "pane": focused_pane.0,
                "session": self
                    .agents_workspace
                    .active_session_in_pane(focused_pane)
                    .map(|session_id| session_id.0),
            }),
        );
        self.dispatch_gpui_workspace_active_session_attention_acknowledge(focused_pane, cx);
        self.dispatch_gpui_workspace_active_session_selected(focused_pane, cx);
        self.scroll_workspace_pane_active_tab(pane_id);
        self.scroll_workspace_pane_active_tab(self.agents_workspace.focused_pane);
        self.persist_shell_layout_state();
    }

    pub(crate) fn dispatch_gpui_workspace_active_session_attention_acknowledge(
        &mut self,
        pane_id: WorkspacePaneId,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(session_id) = self.agents_workspace.active_session_in_pane(pane_id) {
            self.dispatch_gpui_workspace_session_attention_acknowledge(session_id, cx);
        }
    }

    pub(crate) fn dispatch_gpui_workspace_active_session_selected(
        &mut self,
        pane_id: WorkspacePaneId,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(session_id) = self.agents_workspace.active_session_in_pane(pane_id) else {
            return;
        };
        let Some(key) = self.workspace_terminal_key_for_shell_session(session_id) else {
            return;
        };
        match key {
            GpuiWorkspaceTerminalSessionKey::Local(key) => {
                self.local_workspace_latest_focus_key = Some(key.clone());
                self.dispatch_gpui_workspace_tab_session_selected(
                    key.project_id.as_str(),
                    key.session_id.as_str(),
                    false,
                    false,
                    cx,
                );
            }
            GpuiWorkspaceTerminalSessionKey::Remote(key) => {
                self.set_sidebar_gxserver_remote_attach_focus_state(&key, cx);
            }
        }
    }

    pub(crate) fn acknowledge_agents_pane_attention_from_chrome_click(
        &mut self,
        pane_id: WorkspacePaneId,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        self.dispatch_gpui_workspace_active_session_attention_acknowledge(pane_id, cx);
        if !self
            .agents_workspace
            .acknowledge_attention_for_active_session_in_pane(pane_id)
        {
            return false;
        }
        self.persist_shell_layout_state();
        cx.notify();
        true
    }

    pub(crate) fn select_agents_tab(
        &mut self,
        pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:FocusRouting 2026-06-26-23:24:
        Agents pane-tab selection follows the macOS click-to-wake setting. The default keeps sleeping tabs cold until their placeholder body is activated; strict `clickToWakeSleepingSessions: false` requests a gxserver wake for mapped sleeping sessions after selection, while unmapped local tabs use the shell-only wake state.
        */
        let settings_snapshot = shared_settings::shared_sidebar_settings_snapshot();
        let selected_local_was_sleeping =
            self.agents_workspace
                .session(session_id)
                .is_some_and(|session| {
                    session.presentation_state == TerminalSessionPresentationState::Sleeping
                });
        let wake_on_tab_selection =
            !gpui_click_to_wake_sleeping_sessions_from_shared_settings(&settings_snapshot)
                && selected_local_was_sleeping;
        let attention_acknowledged = self
            .agents_workspace
            .session(session_id)
            .is_some_and(|session| session.activity == AgentTerminalActivity::Attention);
        self.agents_workspace.select_tab(pane_id, session_id);
        self.dispatch_gpui_workspace_session_attention_acknowledge(session_id, cx);
        self.focus_shell_target(
            ShellFocusTarget::AgentsPane(self.agents_workspace.focused_pane),
            cx,
        );
        self.scroll_workspace_pane_active_tab(pane_id);
        self.persist_shell_layout_state();
        if let Some(key) = self.local_workspace_key_for_shell_session(session_id) {
            let selected_local_runtime_missing =
                self.agents_tab_selected_local_runtime_missing(pane_id, session_id);
            if selected_local_runtime_missing {
                support_logs::append(
                    support_logs::GpuiSupportLog::TerminalFocus,
                    "gpui.terminalFocus.tabSelectedRuntimeMissing",
                    serde_json::json!({
                        "projectId": key.project_id.as_str(),
                        "sessionId": key.session_id.as_str(),
                    }),
                );
            }
            self.dispatch_gpui_workspace_tab_session_selected(
                key.project_id.as_str(),
                key.session_id.as_str(),
                selected_local_was_sleeping,
                selected_local_runtime_missing,
                cx,
            );
        } else if let Some(GpuiWorkspaceTerminalSessionKey::Remote(key)) =
            self.workspace_terminal_key_for_shell_session(session_id)
        {
            if !selected_local_was_sleeping
                && self.agents_tab_selected_local_runtime_missing(pane_id, session_id)
            {
                let _ = self.request_gpui_remote_attach_terminal_open(
                    GpuiRemoteAttachSessionReference {
                        remote_machine_id: key.remote_machine_id,
                        project_id: key.project_id,
                        session_id: key.session_id,
                    },
                    Some(pane_id),
                    AgentsWorkspaceNewTerminalPlacement::Tab,
                    cx,
                );
            } else {
                self.set_sidebar_gxserver_remote_attach_focus_state(&key, cx);
            }
        } else if let Some(key) = self.agents_chat_remote_key_for_session(session_id) {
            self.set_sidebar_gxserver_remote_attach_focus_state(&key, cx);
        }
        if wake_on_tab_selection {
            if self.agents_terminal_session_is_mapped_sleeping(session_id) {
                let _ = self.request_mapped_sleeping_agents_terminal_wake(
                    pane_id,
                    session_id,
                    GpuiLocalWorkspaceLifecycleMutationKind::DirectWake,
                    cx,
                );
            } else if self
                .agents_workspace
                .set_session_sleeping(session_id, false)
            {
                #[cfg(target_os = "windows")]
                self.make_unmapped_windows_agents_wake_startup_eligible(session_id);
                self.persist_shell_layout_state();
                self.sync_gpui_keep_awake_automation_from_current_settings(cx);
            }
        }
        if attention_acknowledged {
            cx.notify();
        }
    }

    pub(crate) fn activate_agents_terminal_placeholder(
        &mut self,
        pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
        cx: &mut gpui::Context<Self>,
    ) {
        if !self
            .agents_workspace
            .session_belongs_to_pane(pane_id, session_id)
        {
            return;
        }

        if let Some(GpuiWorkspaceTerminalSessionKey::Remote(key)) =
            self.workspace_terminal_key_for_shell_session(session_id)
        {
            let _ = self.request_gpui_remote_attach_terminal_open(
                GpuiRemoteAttachSessionReference {
                    remote_machine_id: key.remote_machine_id,
                    project_id: key.project_id,
                    session_id: key.session_id,
                },
                Some(pane_id),
                AgentsWorkspaceNewTerminalPlacement::Tab,
                cx,
            );
            return;
        }

        if self.agents_terminal_session_is_mapped_sleeping(session_id) {
            if !self.request_mapped_sleeping_agents_terminal_wake(
                pane_id,
                session_id,
                GpuiLocalWorkspaceLifecycleMutationKind::DirectWake,
                cx,
            ) {
                return;
            }
            self.dispatch_gpui_workspace_session_attention_acknowledge(session_id, cx);
            let attention_acknowledged = self
                .agents_workspace
                .acknowledge_attention_for_session_activation(session_id);
            /*
            CDXC:SessionSleep 2026-06-26-23:24:
            Placeholder body activation for mapped sleeping gxserver sessions must be a wake request first, matching macOS. Keep the native pane focused while pending, but do not move the tab to Mounting until the sidebar lifecycle result confirms gxserver wake.
            */
            let focus = ShellFocusTarget::AgentsPane(self.agents_workspace.focused_pane);
            let shell_focus_changed = self.shell_focus != focus;
            self.focus_shell_target(focus, cx);
            if shell_focus_changed || attention_acknowledged {
                self.scroll_workspace_pane_active_tab(pane_id);
                self.persist_shell_layout_state();
                cx.notify();
            }
            return;
        }

        #[cfg(target_os = "windows")]
        let was_unmapped_sleeping_windows_terminal = self
            .agents_workspace
            .session(session_id)
            .is_some_and(|session| {
                session.presentation_state == TerminalSessionPresentationState::Sleeping
            });
        let model_changed = activate_agents_terminal_placeholder_with_runtime_attempt_identity(
            &mut self.agents_workspace,
            &mut self.agents_terminal_runtime_sessions,
            pane_id,
            session_id,
        );
        #[cfg(target_os = "windows")]
        if was_unmapped_sleeping_windows_terminal {
            self.make_unmapped_windows_agents_wake_startup_eligible(session_id);
        }
        self.dispatch_gpui_workspace_session_attention_acknowledge(session_id, cx);
        let attention_acknowledged = self
            .agents_workspace
            .acknowledge_attention_for_session_activation(session_id);
        let focus = ShellFocusTarget::AgentsPane(self.agents_workspace.focused_pane);
        let shell_focus_changed = self.shell_focus != focus;
        self.focus_shell_target(focus, cx);
        if model_changed || shell_focus_changed || attention_acknowledged {
            self.scroll_workspace_pane_active_tab(pane_id);
            self.persist_shell_layout_state();
            cx.notify();
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn make_unmapped_windows_agents_wake_startup_eligible(
        &mut self,
        session_id: TerminalSessionId,
    ) {
        /*
        The composited Windows engine tears down an unmapped local child when
        its tab sleeps. There is therefore no parked native owner to reattach:
        waking must enter the normal startup pipeline and create a fresh WSL
        or PowerShell process. Mapped gxserver sessions never reach this path;
        their wake remains daemon-owned and persistent.
        */
        if self
            .local_workspace_key_for_shell_session(session_id)
            .is_some()
        {
            return;
        }
        self.agents_workspace
            .make_mounting_session_startup_eligible(session_id);
    }
}
