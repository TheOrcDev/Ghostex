//! A bot's "+", its Edit SOUL and Edit config, and the sync that brings the bot projects up to
//! date.
//!
//! The launch is the ordinary Hermes launch into the bot's project: gxserver swaps in the project's
//! own `hermes -p <profile>` command (server/src/bot_projects.rs). It skips only what belongs to the
//! agent launcher, which a bot has none of: the launcher's highlighted agent is left alone.

use ghostex_gx_core::MachineId;
use serde_json::{Value, json};

use super::super::gx_rpc;
use crate::GhostexGpuiApp;

impl GhostexGpuiApp {
    /// Asks gxserver to add a project for every Hermes profile that has none yet. The new rows
    /// arrive through the presentation like any added project, so nothing waits on the answer;
    /// gxserver itself refuses while Bots is switched off.
    pub(crate) fn gx_store_sync_bot_projects(&mut self, cx: &mut gpui::Context<Self>) {
        cx.spawn(async move |_, _| {
            let _ = gx_rpc(None, "/api/syncBotProjects", json!({})).await;
        })
        .detach();
    }

    /// An `openBotFile` sidebar message: the bot's own project comes forward and the file opens
    /// in its Code view. The paths come from gx-core's bot row (sidebar_menu/header.rs).
    pub(super) fn gx_store_open_bot_file_message(
        &mut self,
        message: &Value,
        cx: &mut gpui::Context<Self>,
    ) {
        let text = |key: &str| message.get(key).and_then(Value::as_str).map(str::to_string);
        let (Some(group_id), Some(project_path), Some(file_path)) =
            (text("groupId"), text("projectPath"), text("filePath"))
        else {
            return;
        };
        self.open_bot_file_in_code_view(group_id, project_path.into(), file_path.into(), cx);
    }

    /// The bot's name when this is a Hermes launch into a local bot, which gxserver runs as the
    /// bot (`bot_agent_config` in server/src/bot_projects.rs); `None` for any other agent.
    pub(super) fn gx_store_bot_name(&self, project_id: &str, agent_id: &str) -> Option<String> {
        if !agent_id.trim().eq_ignore_ascii_case("hermes-agent") {
            return None;
        }
        let project = self
            .gx_store
            .core
            .presentation()
            .loaded(&MachineId::Local)?
            .project(project_id)?;
        project
            .bot_profile
            .is_some()
            .then(|| project.title.trim().to_string())
            .filter(|name| !name.is_empty())
    }

    /// A `runSidebarBot` host message.
    pub(super) fn gx_store_run_sidebar_bot_message(
        &mut self,
        message: &Value,
        cx: &mut gpui::Context<Self>,
    ) {
        let group_id = message.get("groupId").and_then(Value::as_str);
        self.gx_store_request_agent_launch("hermes-agent", group_id, None, cx);
    }
}
