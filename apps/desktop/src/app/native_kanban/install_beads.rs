//! The board notice's Install Beads button: installs Beads through gxserver's managed tools on the
//! machine that runs the project (the remote computer for a remote project), then reloads the
//! board.

use std::time::Duration;

use gpui::Context;

use crate::GhostexGpuiApp;
use crate::app::helpers::{
    gpui_managed_tool_progress_line, gpui_managed_tool_read, gpui_managed_tool_run,
};

use super::state::KanbanRefreshMode;

impl GhostexGpuiApp {
    /// Reads gxserver's managed Beads tool once per board, for the button's tooltip and for the
    /// reason Beads cannot be installed on this machine (glibc older than 2.34, for example).
    pub(crate) fn native_kanban_fetch_beads_tool(&mut self, cx: &mut Context<Self>) {
        if self.native_kanban.beads_install.requested {
            return;
        }
        let Some(context) = self.native_kanban_bridge_context() else {
            return;
        };
        self.native_kanban.beads_install.requested = true;
        let generation = self.native_kanban.generation;
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let answer = background
                .spawn(
                    async move { gpui_managed_tool_read(context.remote_target.as_ref(), "beads") },
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.native_kanban.generation != generation {
                    return;
                }
                let install = &mut this.native_kanban.beads_install;
                match answer {
                    Ok(state) => {
                        install.plan = state["installPlan"].as_str().map(str::to_string);
                        install.blocked_reason = if state["supported"] == false {
                            state["unsupportedReason"].as_str().map(str::to_string)
                        } else {
                            state["unavailableReason"].as_str().map(str::to_string)
                        };
                    }
                    Err(error) => {
                        install.blocked_reason = Some(format!(
                            "Ghostex can't install Beads on this machine: {error}"
                        ));
                    }
                }
                this.native_kanban_notify(cx);
            });
        })
        .detach();
    }

    /// CDXC:ManagedTools 2026-09-29 DECISION:
    /// User (6B): the Project board gets a one-click Beads install instead of only instructions. The notice's Install Beads button installs Beads from its official GitHub releases through gxserver on the machine that runs the project, shows the install's progress, and reloads the board when it succeeds.
    pub(crate) fn native_kanban_install_beads(&mut self, cx: &mut Context<Self>) {
        if self.native_kanban.beads_install.running {
            return;
        }
        let Some(context) = self.native_kanban_bridge_context() else {
            return;
        };
        let generation = self.native_kanban.generation;
        let install = &mut self.native_kanban.beads_install;
        install.running = true;
        install.error = None;
        if let Ok(mut progress) = install.progress.lock() {
            *progress = Some("Starting the Beads install…".to_string());
        }
        let progress = install.progress.clone();
        self.native_kanban_notify(cx);
        let background = cx.background_executor().clone();
        let pump = background.clone();
        cx.spawn(async move |this, cx| {
            loop {
                pump.timer(Duration::from_secs(1)).await;
                let still_running = this
                    .update(cx, |this, cx| {
                        let running = this.native_kanban.generation == generation
                            && this.native_kanban.beads_install.running;
                        if running {
                            this.native_kanban_notify(cx);
                        }
                        running
                    })
                    .unwrap_or(false);
                if !still_running {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let result = background
                .spawn(async move {
                    gpui_managed_tool_run(
                        context.remote_target.as_ref(),
                        "beads",
                        "install",
                        |state| {
                            if let (Ok(mut progress), Some(line)) =
                                (progress.lock(), gpui_managed_tool_progress_line(state))
                            {
                                *progress = Some(line);
                            }
                        },
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.native_kanban.generation != generation {
                    return;
                }
                let install = &mut this.native_kanban.beads_install;
                install.running = false;
                match result {
                    Ok(_) => {
                        install.error = None;
                        this.dispatch_gpui_app_modal_toast(
                            "success",
                            "Beads installed",
                            "The Project board is loading.",
                            cx,
                        );
                        this.native_kanban_refresh(KanbanRefreshMode::Manual, cx);
                    }
                    Err(error) => install.error = Some(error),
                }
                this.native_kanban_notify(cx);
            });
        })
        .detach();
    }
}
