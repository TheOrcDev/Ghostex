//! Running a Ghostex-managed tool's install in a command-pane terminal, for installs that need a
//! password prompt where there is no desktop dialog (Linux system tools on WSL).

use std::time::Duration;

use gpui::Window;

use crate::app::helpers::*;
use crate::*;

impl GhostexGpuiApp {
    /// CDXC:ManagedTools 2026-09-29 WHY:
    /// Settings > Integrations > Tools installs Linux system tools with one password prompt; where polkit has no dialog to show (WSL, a headless session) gxserver answers with the `sudo` command instead, and the prompt has to be a real terminal. The page sends only the tool id: the command is read from gxserver here, so a page can never make the app run text of its choosing.
    /// SEE-ALSO: server/src/managed_tools/system_tools.rs, packages/core-ui/settings-modal/tabs/managed-tools-section.tsx (deleted 2026-10-01).
    pub(crate) fn run_managed_tool_terminal_command(
        &mut self,
        tool_id: String,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let background = cx.background_executor().clone();
        let lookup = tool_id.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = background
                .spawn(async move {
                    gpui_gxserver_rpc_result(
                        "/api/managedTools",
                        &serde_json::json!({ "action": "read", "tool": lookup }),
                        Duration::from_secs(30),
                    )
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                let state = match result {
                    Ok(state) => state,
                    Err(error) => {
                        this.dispatch_gpui_app_modal_toast(
                            "error",
                            "Couldn't start the install",
                            &error,
                            cx,
                        );
                        return;
                    }
                };
                let label = state["label"].as_str().unwrap_or("tools").to_string();
                let Some(command) = state["terminalCommand"]
                    .as_str()
                    .filter(|command| !command.trim().is_empty())
                else {
                    this.dispatch_gpui_app_modal_toast(
                        "info",
                        &format!("{label} doesn't need a terminal"),
                        "Use its Install button in Settings > Integrations > Tools.",
                        cx,
                    );
                    return;
                };
                let title = if tool_id == "systemTools" {
                    "Install system tools".to_string()
                } else {
                    format!("Install {label}")
                };
                this.open_gpui_command_action_terminal(
                    format!("ghostex.gpui.installManagedTool.{tool_id}"),
                    title,
                    command.to_string(),
                    false,
                    false,
                    window,
                    cx,
                );
                this.dispatch_gpui_app_modal_toast(
                    "info",
                    &format!("Installing {label}"),
                    "Type your password in the terminal tab. Refresh Settings > Integrations > Tools when it finishes.",
                    cx,
                );
            });
        })
        .detach();
    }
}
