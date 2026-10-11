//! Full Reload, performed by the store.
//!
//! CDXC:Sessions 2026-09-21 WHY:
//! It does not call the daemon itself. Full Reload is the sleep and the wake the store already
//! owns, run ONE AT A TIME because the TypeScript awaited the first before it started the second
//! and because a wake that overtakes its own sleep reloads nothing. So this file waits, and every
//! decision it acts on is gx-core's.
//!
//! **The counters that prove this path fires in the app** are `reloads`, `reloadLegs` and
//! `remounts` on `gxStore.sidebarLifecycle`. A run where the user used Advanced > Full Reload and
//! any of them is zero means the command never reached here, which is the failure the envelope bug
//! of piece 3d was.
//!
//! SEE-ALSO: packages/gx-core/src/sidebar_actions/reload.rs.

use ghostex_gx_core::{
    owns_reload_message, owns_reload_set_message, owns_remote_session_message, plan_full_reload,
    plan_reload_set, reload_continues_after,
};
use serde_json::Value;

use crate::GhostexGpuiApp;

impl GhostexGpuiApp {
    /// Answers `fullReloadSession` and `restartSession` when the store owns the row.
    pub(crate) fn gx_store_run_sidebar_reload(
        &mut self,
        command: &Value,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some(message) = wrapped_message(command) else {
            return false;
        };
        if !owns_reload_message(message) {
            return false;
        }
        if !self.gx_store_sidebar_list_ready() {
            self.gx_store.sidebar_lifecycle.declined_source += 1;
            return false;
        }
        match self.gx_store_start_reload(message, cx) {
            Some(task) => {
                task.detach();
                true
            }
            None => false,
        }
    }

    /// The same Full Reload as a task that resolves to whether it came home without a failed call,
    /// which is what a set reload waits on before it moves to the next row. `None` means the
    /// single-session planner does not own the row.
    fn gx_store_start_reload(
        &mut self,
        message: &Value,
        cx: &mut gpui::Context<Self>,
    ) -> Option<gpui::Task<bool>> {
        // A remote row, a browser row and an id that does not parse are refused inside the
        // planner, each one for a reason written down there.
        let plan = plan_full_reload(&self.gx_store.core, message)?;
        self.gx_store.sidebar_lifecycle.reloads += 1;
        self.gx_store.sidebar_lifecycle.reload_legs += plan.legs.len() as u64;
        self.gx_store
            .diagnostics
            .sidebar_reload_ran(&plan, self.gx_store.sidebar_lifecycle);
        let legs = plan.legs;
        let session = plan.session;
        // Held until the wake has come home, so the workspace keeps this row's tab (and with it the
        // focus) while the sleep kills its attach client (`CDXC:Sessions 2026-10-03 DECISION` in
        // gx-core reload.rs).
        self.gx_store.reloading_sessions.push(session.clone());
        Some(cx.spawn(async move |this, cx| {
            let mut completed = true;
            for leg in legs {
                // Each leg goes through the single-session path, which owns the call, the declined
                // leg, the replacement focus and the echo guard. The wait is for the ANSWER and not
                // for a timer: the provider has to be dead before it is asked to come back, and a
                // sleep whose call failed stops the reload, as the TypeScript's first `await`
                // rejecting did.
                let started = this.update(cx, |this, cx| this.gx_store_start_lifecycle(&leg, cx));
                let Ok(Some(task)) = started else {
                    completed = false;
                    break;
                };
                if !reload_continues_after(task.await) {
                    let _ = this.update(cx, |this, _| {
                        this.gx_store.sidebar_lifecycle.reloads_stopped += 1;
                    });
                    completed = false;
                    break;
                }
            }
            let _ = this.update(cx, |this, _| {
                let reloading = &mut this.gx_store.reloading_sessions;
                if let Some(index) = reloading.iter().position(|held| *held == session) {
                    reloading.remove(index);
                }
            });
            completed
        }))
    }

    /// Answers `fullReloadProjectZmxSessions` and `fullReloadGroup`: a Full Reload of each row of
    /// the set, ONE AT A TIME, stopping at the first that fails (gx-core `reload_set.rs`).
    ///
    /// Both are wrapped gxserver messages (`{ type: 'command', message }`), posted by the project
    /// menu and the user-made group menu.
    pub(crate) fn gx_store_run_sidebar_reload_set(
        &mut self,
        command: &Value,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some(message) = wrapped_message(command) else {
            return false;
        };
        if !owns_reload_set_message(message) {
            return false;
        }
        if !self.gx_store_sidebar_list_ready() {
            self.gx_store.sidebar_lifecycle.declined_source += 1;
            return false;
        }
        // A user-made group's members are the workspace session groups document, so its stored key
        // has to be in hand first, for the reason every edit of it waits: an answer computed against
        // a document this app has not read would reload the wrong rows, or none.
        if message.get("type").and_then(Value::as_str) == Some("fullReloadGroup")
            && !self.gx_store_restore_workspace_groups(cx)
        {
            self.gx_store.sidebar_lifecycle.declined_source += 1;
            return false;
        }
        let plan = {
            let store = &self.gx_store;
            plan_reload_set(&store.core, store.workspace_groups.sync.document(), message)
        };
        let Some(plan) = plan else {
            return false;
        };
        self.gx_store.sidebar_lifecycle.reload_sets += 1;
        self.gx_store.sidebar_lifecycle.reload_set_rows += plan.messages.len() as u64;
        cx.spawn(async move |this, cx| {
            let mut index = 0;
            while let Some(message) = plan.messages.get(index) {
                let started = this.update(cx, |this, cx| {
                    // The route the dispatcher takes for this row: a remote row down its machine's
                    // tunnel, a local one through the single-session reload, and anything neither
                    // owns to the old runtime, which is the one row nothing here can wait for.
                    if owns_remote_session_message(message) {
                        if let Some(task) = this.gx_store_start_remote(message, cx) {
                            return Some(task);
                        }
                    } else if let Some(task) = this.gx_store_start_reload(message, cx) {
                        return Some(task);
                    }
                    this.dispatch_native_sidebar_command(message.clone(), cx);
                    None
                });
                let completed = match started {
                    Err(_) => return,
                    Ok(Some(task)) => task.await,
                    Ok(None) => true,
                };
                match plan.step_after(index, completed) {
                    Some(next) => index = next,
                    None => {
                        if !completed {
                            let _ = this.update(cx, |this, _| {
                                this.gx_store.sidebar_lifecycle.reload_sets_stopped += 1;
                            });
                        }
                        return;
                    }
                }
            }
        })
        .detach();
        true
    }
}

/// The inner message of a gxserver command envelope. Both of these payloads are wrapped ones, not
/// renderer commands: they reach the runtime through `controller.ts:131`, unlike `sessionAction`
/// and `batch`, which arrive at the top level.
fn wrapped_message(command: &Value) -> Option<&Value> {
    if command.get("type").and_then(Value::as_str) != Some("command") {
        return None;
    }
    command.get("message")
}
