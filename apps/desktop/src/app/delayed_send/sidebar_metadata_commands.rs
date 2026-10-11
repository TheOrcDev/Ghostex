//! Sidebar agent and command metadata writes, deleted-action sessions and pinned prompt saves.

// RefCell backs cross-platform runtime state (window frame persistence), not
// just the macOS-only shims that first introduced the import.

use anyhow::Result;

use crate::app::helpers::*;
use crate::app::model::*;
use crate::*;

struct GpuiDeletedActionSession {
    session_id: CommandSessionId,
    command_id: String,
    scope: GpuiSidebarCommandScope,
    local_key: Option<GpuiLocalWorkspaceSessionKey>,
    remote_reference: Option<GpuiRemoteAttachSessionReference>,
    run_id: Option<String>,
}

impl GhostexGpuiApp {
    pub(crate) fn handle_gpui_sidebar_agent_metadata_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:AgentLauncher 2026-06-24-20:54:
        Settings > Agents writes in GPUI must enter gxserver's semantic agent/action mutation contract. The app-modal bridge only validates the bounded CEF message shape; gxserver owns hidden-default restoration, custom metadata persistence, order normalization, and refreshed HUD/project rows without logging launcher text, project identity, paths, URLs, tokens, stdout, or stderr.
        */
        let write = match gpui_sidebar_agent_metadata_write_from_command(command) {
            Ok(write) => write,
            Err(_) => {
                self.dispatch_gpui_sidebar_metadata_write_failure(
                    GpuiSidebarMetadataWriteKind::Agents,
                    GPUI_SIDEBAR_METADATA_GENERIC_ERROR,
                    cx,
                );
                return;
            }
        };
        let failure_order_sync_result = write.order_sync_result("error", Vec::new());
        let active_project_id = self.gpui_app_modal_active_project_id();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let result =
                background
                    .spawn(async move {
                        gpui_apply_sidebar_agent_metadata_write(write, active_project_id)
                    })
                    .await;
            let _ = this.update(cx, |this, cx| {
                this.finish_gpui_sidebar_metadata_write(
                    GpuiSidebarMetadataWriteKind::Agents,
                    result,
                    failure_order_sync_result,
                    cx,
                );
            });
        })
        .detach();
    }

    pub(crate) fn handle_gpui_sidebar_command_metadata_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:AgentLauncher 2026-06-24-20:54:
        Settings > Actions writes in GPUI are active-project scoped but gxserver-owned. Pass the current app-modal active project id into the semantic mutation contract so gxserver resolves worktree parent ownership, deleted default actions, command/browser validation, display order, and refreshed HUD/project rows without local metadata rewrites.
        */
        let active_project_id = self.gpui_app_modal_active_project_id();
        let write =
            match gpui_sidebar_command_metadata_write_from_command(command, active_project_id) {
                Ok(write) => write,
                Err(_) => {
                    self.dispatch_gpui_sidebar_metadata_write_failure(
                        GpuiSidebarMetadataWriteKind::Commands,
                        GPUI_SIDEBAR_METADATA_GENERIC_ERROR,
                        cx,
                    );
                    return;
                }
            };
        let failure_order_sync_result = write.order_sync_result("error", Vec::new());
        let remote_target = (write.scope() == GpuiSidebarCommandScope::Project)
            .then(|| {
                gpui_remote_project_reference_from_project_id(
                    gpui_sidebar_command_write_active_project_id(&write),
                )
            })
            .flatten()
            .and_then(|project| {
                self.gpui_remote_gxserver_request_target(&project.remote_machine_id)
            });
        let deleted_session = self.gpui_session_for_deleted_action(&write);
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let result = background
                .spawn(
                    async move { gpui_apply_sidebar_command_metadata_write(write, remote_target) },
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                if result.is_ok()
                    && let Some(deleted_session) = deleted_session
                {
                    this.close_gpui_deleted_sidebar_command_session(deleted_session, cx);
                }
                this.finish_gpui_sidebar_metadata_write(
                    GpuiSidebarMetadataWriteKind::Commands,
                    result,
                    failure_order_sync_result,
                    cx,
                );
            });
        })
        .detach();
    }

    /// CDXC:AgentLauncher 2026-09-23 WHY:
    /// Built-in Action IDs repeat across projects and computers. A delete captures only its proven owner tab, then verifies that same run after the daemon accepts the mutation; failed writes and newer runs must keep their terminals.
    fn gpui_session_for_deleted_action(
        &self,
        write: &GpuiSidebarCommandMetadataWrite,
    ) -> Option<GpuiDeletedActionSession> {
        let command_id = write.deleted_command_id()?;
        self.command_pane
            .flat_tab_ids()
            .into_iter()
            .find_map(|(_, session_id)| {
                let session = self.command_pane.session(session_id)?;
                if session.action_command_id.as_deref() != Some(command_id)
                    || session.action_scope != Some(write.scope())
                {
                    return None;
                }
                let local_key = session.gxserver_session_key.clone();
                let remote_reference =
                    self.command_remote_action_session_for_command_tab(session_id);
                if write.scope() == GpuiSidebarCommandScope::Project {
                    let project_id = gpui_sidebar_command_write_active_project_id(write);
                    let matches = match gpui_remote_project_reference_from_project_id(project_id) {
                        Some(project) => remote_reference.as_ref().is_some_and(|session| {
                            session.remote_machine_id == project.remote_machine_id
                                && session.project_id == project.project_id
                        }),
                        None => {
                            remote_reference.is_none()
                                && local_key
                                    .as_ref()
                                    .is_some_and(|key| key.project_id == project_id)
                        }
                    };
                    if !matches {
                        return None;
                    }
                }
                Some(GpuiDeletedActionSession {
                    session_id,
                    command_id: command_id.to_string(),
                    scope: write.scope(),
                    local_key,
                    remote_reference,
                    run_id: session.action_run_id.clone(),
                })
            })
    }

    fn close_gpui_deleted_sidebar_command_session(
        &mut self,
        deleted: GpuiDeletedActionSession,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let session_id = deleted.session_id;
        if self.command_remote_action_session_for_command_tab(session_id)
            != deleted.remote_reference
        {
            return false;
        }
        let Some((group_id, _)) = self
            .command_pane
            .flat_tab_ids()
            .into_iter()
            .find(|(_, id)| *id == session_id)
        else {
            return false;
        };
        let Some(session) = self.command_pane.session_mut(session_id) else {
            return false;
        };
        if session.action_command_id.as_deref() != Some(deleted.command_id.as_str())
            || session.action_scope != Some(deleted.scope)
            || session.gxserver_session_key != deleted.local_key
            || session.action_run_id != deleted.run_id
        {
            return false;
        }
        session.activity = CommandTerminalActivity::Idle;
        session.action_command_id = None;
        session.action_scope = None;
        session.action_close_terminal_on_exit = false;
        session.action_play_completion_sound = false;
        session.action_run_id = None;
        session.action_status_file_path = None;
        self.clear_gpui_command_delayed_send_timer(session_id);
        self.clear_gpui_command_close_after_done_timer(session_id);
        if !self
            .command_pane
            .close_session_from_direct_tab_close(group_id, session_id)
        {
            return false;
        }
        self.forget_command_gxserver_session_for_closed_tab(session_id, cx);
        self.clear_command_resize_hover_state_if_command_pane_hidden();
        if !self.command_pane.has_sessions() && self.shell_focus == ShellFocusTarget::CommandPane {
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

    pub(crate) fn finish_gpui_sidebar_metadata_write(
        &mut self,
        kind: GpuiSidebarMetadataWriteKind,
        result: Result<Option<serde_json::Value>, String>,
        failure_order_sync_result: Option<serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        match result {
            Ok(order_sync_result) => {
                self.refresh_open_gpui_app_modal_sidebar_state_in_background(cx);
                self.refresh_titlebar_actions_in_background(cx);
                if let Some(order_sync_result) = order_sync_result {
                    self.dispatch_open_gpui_app_modal_message(order_sync_result, cx);
                }
                cx.notify();
            }
            Err(error_code) => {
                if let Some(order_sync_result) = failure_order_sync_result {
                    self.dispatch_open_gpui_app_modal_message(order_sync_result, cx);
                }
                self.dispatch_gpui_sidebar_metadata_write_failure(kind, error_code.as_str(), cx);
            }
        }
    }

    pub(crate) fn dispatch_gpui_sidebar_metadata_write_failure(
        &mut self,
        kind: GpuiSidebarMetadataWriteKind,
        error_code: &str,
        cx: &mut gpui::Context<Self>,
    ) {
        if error_code == GPUI_SIDEBAR_DUPLICATE_ACTION_TITLE_ERROR {
            self.dispatch_gpui_app_modal_toast(
                "warning",
                "Action title already exists",
                "An action with that title already exists in this project.",
                cx,
            );
            return;
        }
        self.dispatch_gpui_app_modal_toast(
            "warning",
            "Settings were not saved",
            kind.failure_message(),
            cx,
        );
    }

    pub(crate) fn handle_gpui_save_pinned_prompt_command(
        &mut self,
        command: &serde_json::Map<String, serde_json::Value>,
        cx: &mut gpui::Context<Self>,
    ) {
        /*
        CDXC:ServerDaemon 2026-06-24-13:30:
        Pinned Prompt saves must use the shared gxserver product-data contract
        while keeping the React `savePinnedPrompt` bridge message unchanged.
        Reject malformed non-string fields locally and let gxserver preserve
        createdAt, stamp updatedAt, normalize titles, and keep prompt text out
        of logs and progress channels.
        */
        let Some(content) = command.get("content").and_then(serde_json::Value::as_str) else {
            return;
        };
        let Some(title) = command.get("title").and_then(serde_json::Value::as_str) else {
            return;
        };
        if command
            .get("promptId")
            .is_some_and(|value| !value.is_null() && !value.is_string())
        {
            return;
        }
        let prompt_id = command
            .get("promptId")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if gpui_save_gxserver_pinned_prompt(content, title, prompt_id.as_deref()).is_ok() {
            self.refresh_open_gpui_app_modal_sidebar_state(
                self.gpui_app_modal_sidebar_state_message(),
                cx,
            );
        }
    }
}
