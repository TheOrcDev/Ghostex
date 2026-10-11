//! Installing a provider's missing CLI (`gh`, `glab`) from its row: the `installSourceControlTool`
//! and `readSourceControlTool` round trips to gxserver's managed tools on the dialog's machine, the
//! poll while the install runs, and a fresh source-control discovery once it succeeds.
//!
//! CDXC:ManagedTools 2026-09-29 DECISION:
//! User (6B): GitHub and GitLab repository rows whose CLI is missing get a one-click install instead of instructions. The row's Setup Required button becomes Install GitHub CLI / Install GitLab CLI; Ghostex downloads the CLI from its official releases on the machine the dialog is adding to, shows the progress in the row, then re-checks the provider so the row moves on to its sign-in state.

use gpui::Context;
use serde_json::Value;

use super::model::AddProjectSourceId;
use super::window::{GpuiAddProjectModalWindow, Pending};

#[derive(Clone, Debug)]
pub(crate) struct ToolInstall {
    pub(crate) machine_id: String,
    pub(crate) source: AddProjectSourceId,
    pub(crate) tool: String,
    pub(crate) running: bool,
    pub(crate) job_id: Option<String>,
    pub(crate) progress: Option<String>,
    pub(crate) error: Option<String>,
}

impl ToolInstall {
    /// The row's description while the install runs, or after it failed.
    pub(crate) fn status_line(&self) -> Option<String> {
        if self.running {
            Some(format!(
                "Installing {}…{}",
                tool_label(&self.tool),
                self.progress
                    .as_ref()
                    .map(|line| format!(" {line}"))
                    .unwrap_or_default()
            ))
        } else {
            self.error
                .as_ref()
                .map(|error| format!("The install did not finish: {error}"))
        }
    }
}

pub(crate) fn tool_label(tool: &str) -> &'static str {
    match tool {
        "glab" => "GitLab CLI",
        _ => "GitHub CLI",
    }
}

fn progress_line(state: &Value) -> Option<String> {
    state["job"]["output"]
        .as_str()?
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

impl GpuiAddProjectModalWindow {
    pub(super) fn start_tool_install(
        &mut self,
        source: AddProjectSourceId,
        tool: String,
        cx: &mut Context<Self>,
    ) {
        if self
            .tool_install
            .as_ref()
            .is_some_and(|install| install.running)
        {
            return;
        }
        let Some(machine_id) = self.derive().machine_id else {
            return;
        };
        self.tool_install = Some(ToolInstall {
            machine_id: machine_id.clone(),
            source,
            tool: tool.clone(),
            running: true,
            job_id: None,
            progress: None,
            error: None,
        });
        self.request(
            "installSourceControlTool",
            &machine_id,
            serde_json::json!({ "tool": tool }),
            Pending::ToolInstall {
                machine_id: machine_id.clone(),
            },
            cx,
        );
        cx.notify();
    }

    fn read_tool_install(&mut self, cx: &mut Context<Self>) {
        let Some(install) = self.tool_install.as_ref().filter(|install| install.running) else {
            return;
        };
        let machine_id = install.machine_id.clone();
        let tool = install.tool.clone();
        self.request(
            "readSourceControlTool",
            &machine_id,
            serde_json::json!({ "tool": tool }),
            Pending::ToolInstall {
                machine_id: machine_id.clone(),
            },
            cx,
        );
    }

    /// An answer to either round trip: the managed tool's state with its install job.
    pub(super) fn receive_tool_install(
        &mut self,
        machine_id: String,
        result: Result<Value, String>,
        cx: &mut Context<Self>,
    ) {
        let Some(install) = self
            .tool_install
            .as_mut()
            .filter(|install| install.running && install.machine_id == machine_id)
        else {
            return;
        };
        let state = match result {
            Ok(value) => value,
            Err(error) => {
                install.running = false;
                install.error = Some(error);
                cx.notify();
                return;
            }
        };
        let job = &state["job"];
        let job_id = job["id"].as_str().map(str::to_string);
        if install.job_id.is_none() {
            install.job_id = job_id.clone();
        }
        let same_job = install.job_id.is_none() || job_id == install.job_id;
        let status = job["status"].as_str().unwrap_or_default();
        if same_job && matches!(status, "queued" | "running") {
            install.progress = progress_line(&state).or(install.progress.take());
            let delay = self.clone_job_poll_interval * 2;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(delay).await;
                let _ = this.update(cx, |this, cx| this.read_tool_install(cx));
            })
            .detach();
            cx.notify();
            return;
        }
        install.running = false;
        if status == "failed" {
            install.error = Some(
                job["error"]
                    .as_str()
                    .filter(|error| !error.trim().is_empty())
                    .map(str::to_string)
                    .or_else(|| progress_line(&state))
                    .unwrap_or_else(|| "The install did not finish.".to_string()),
            );
            cx.notify();
            return;
        }
        // Installed: forget this machine's discovery so the Sources step probes it again and the
        // row shows the provider's sign-in state.
        self.tool_install = None;
        self.discovery.remove(&machine_id);
        cx.notify();
    }
}
