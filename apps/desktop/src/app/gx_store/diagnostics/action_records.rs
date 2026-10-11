use std::time::Instant;

use serde_json::{Value, json};

use super::*;

impl GxStoreDiagnostics {
    /// One line per sleep or wake the store performed: which call it made, what the daemon said,
    /// how long the round trip took, and the run's totals.
    ///
    /// No id and no title: the call name and the answer are a fixed vocabulary, and everything
    /// else a lifecycle payload carries is a project id or a session id. `roundTripMs` is the
    /// daemon's, not ours, and it is the number that says whether the optimistic value was ever
    /// on screen: a round trip under a frame means the daemon answered before the user could see
    /// anything, and a long one is the window the overlay exists for.
    pub(in crate::app::gx_store) fn sidebar_lifecycle_ran(
        &mut self,
        request: &ghostex_gx_core::LifecycleRequest,
        answer: &str,
        round_trip_ms: u64,
        counters: super::sidebar_lifecycle::SidebarLifecycleCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarLifecycle",
            json!({
                "call": log_text(request.call.as_str()),
                "answer": log_text(answer),
                "roundTripMs": round_trip_ms,
                "hadReplacementFocus": request.replacement_focus.is_some(),
                "sleeps": counters.sleeps,
                "wakes": counters.wakes,
                "accepted": counters.accepted,
                "declined": counters.declined,
                "failed": counters.failed,
                "alreadyAgreed": counters.already_agreed,
                "focusFollowUps": counters.focus_follow_ups,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// One line per `batch` envelope answered: how many messages it posted and whether it cleared
    /// the multi-selection. No ids: a batch names every selected row.
    pub(in crate::app::gx_store) fn sidebar_batch_ran(
        &mut self,
        messages: usize,
        cleared_selection: bool,
        counters: super::sidebar_bulk::SidebarBulkCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarBatch",
            json!({
                "messages": messages as u64,
                "clearedSelection": cleared_selection,
                "batches": counters.batches,
                "batchMessages": counters.batch_messages,
                "batchesClearing": counters.batches_clearing,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// One line per plural payload answered: which action, how many rows it resolved, and whether
    /// the fan-out is paced. No ids and no project: the counts are what a support log needs.
    ///
    /// `rows` is the number to read. A project Wake that resolves zero is a project with nothing
    /// asleep in it, which is correct; a Sleep Selected that resolves zero when rows were selected
    /// is not, and only this line can tell the two apart.
    /// A Full Reload, named by its legs rather than by a label: the record says how many went out
    /// and which row they were for, never what the session is.
    pub(in crate::app::gx_store) fn sidebar_reload_ran(
        &mut self,
        plan: &ghostex_gx_core::ReloadPlan,
        counters: super::sidebar_lifecycle::SidebarLifecycleCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarReload",
            json!({
                "legs": plan.legs.len() as u64,
                "reloads": counters.reloads,
                "reloadLegs": counters.reload_legs,
                "reloadsStopped": counters.reloads_stopped,
                "remounts": counters.remounts,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// A drag, named by what it posted and never by the row it moved. The refusal reason is a
    /// fixed word from a closed list, so it can say WHY nothing happened without carrying an id.
    ///
    /// CDXC:Sidebar 2026-09-21 WHY:
    /// Every value here is a count or one of a handful of fixed words, which is what keeps it
    /// through `sanitize_json_value`: the sanitizer caps depth at 4, redacts a string over 120
    /// characters or containing a slash, and silently drops an object past 32 entries. An order is
    /// a list of session ids, each one containing a colon and a project path fragment, so it is
    /// reported as a LENGTH and never as itself.
    pub(in crate::app::gx_store) fn sidebar_move_ran(
        &mut self,
        plan: &ghostex_gx_core::SessionMovePlan,
        counters: super::sidebar_drag::SidebarDragCounters,
    ) {
        if self.sidebar_drag_records >= MAX_SIDEBAR_ACTION_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.sidebar_drag_records += 1;
        record(
            "gxStore.sidebarDrag",
            json!({
                "posts": plan.messages.len() as u64,
                "refusal": plan.refusal.unwrap_or("none"),
                "moves": counters.moves,
                "movePosts": counters.move_posts,
                "moveRefusals": counters.move_refusals,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// What one order message wrote. `kind` is the message type, which is one of three fixed
    /// strings, and `rows` is how long the order was, never the order itself.
    pub(in crate::app::gx_store) fn sidebar_order_write_ran(
        &mut self,
        message: &Value,
        plan: &ghostex_gx_core::OrderWritePlan,
        counters: super::sidebar_drag::SidebarDragCounters,
    ) {
        if self.sidebar_drag_records >= MAX_SIDEBAR_ACTION_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.sidebar_drag_records += 1;
        record(
            "gxStore.sidebarOrderWrite",
            json!({
                "kind": message.get("type").and_then(Value::as_str).unwrap_or("?"),
                "rows": message
                    .get("sessionIds")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0) as u64,
                "writes": plan.writes.len() as u64,
                "refusal": plan.refusal.unwrap_or("none"),
                "orderWrites": counters.order_writes,
                "documentEdits": counters.document_edits,
                "sessionOrderCalls": counters.session_order_calls,
                "activations": counters.activations,
                "toasts": counters.toasts,
            }),
        );
    }

    /// A push of the workspace session groups document, and what the guard has seen so far.
    /// `echoesRefused` is the guard doing its job; a run with edits and a zero there means the
    /// window between an edit and its push never opened.
    pub(in crate::app::gx_store) fn workspace_groups_pushed(
        &mut self,
        ok: bool,
        counters: super::workspace_groups::WorkspaceGroupsCounters,
    ) {
        self.workspace_groups_record(Some(ok), counters, None);
    }

    /// The same counters on the periodic path, whether or not anything has happened.
    ///
    /// CDXC:Sessions 2026-09-21 WHY:
    /// **A record that is emitted at ONE instant is a record that is not there when it is read.**
    /// This one was emitted only from a push, so a run with no group edit had no line; it was then
    /// also emitted from the reconcile, which fires once, at about a second into the run, and two
    /// live rounds produced no line either, because `routine_logging_enabled()` reads the shared
    /// settings snapshot and everything through `record()` is silent until that snapshot is warm,
    /// while `gxStore.loaded` is not (it calls `append` directly, which is why it was always there
    /// to mislead). So the counters ride the SAME periodic path as `gxStore.sidebarShadow.summary`,
    /// which is proved to reach the log in a quiet run, and the first line is emitted even when
    /// every counter is zero, because "the path never ran" is the answer that was missing twice.
    pub(in crate::app::gx_store) fn workspace_groups_summary(
        &mut self,
        counters: super::workspace_groups::WorkspaceGroupsCounters,
        side_state_held: bool,
    ) {
        if self
            .workspace_groups_summary_written
            .is_some_and(|written| written == (counters, side_state_held))
            || self
                .workspace_groups_summary_at
                .is_some_and(|at| at.elapsed() < PERIODIC_SUMMARY_INTERVAL)
        {
            return;
        }
        self.workspace_groups_summary_at = Some(Instant::now());
        if !routine_logging_enabled() {
            return;
        }
        self.workspace_groups_summary_written = Some((counters, side_state_held));
        self.workspace_groups_record(None, counters, Some(side_state_held));
    }

    fn workspace_groups_record(
        &mut self,
        ok: Option<bool>,
        counters: super::workspace_groups::WorkspaceGroupsCounters,
        side_state_held: Option<bool>,
    ) {
        if self.workspace_groups_records >= MAX_SIDEBAR_ACTION_RECORDS || !routine_logging_enabled()
        {
            return;
        }
        self.workspace_groups_records += 1;
        record(
            "gxStore.workspaceGroups",
            json!({
                "ok": ok,
                "edits": counters.edits,
                "storageWrites": counters.storage_writes,
                "storageRemoves": counters.storage_removes,
                "storageAttempts": counters.storage_attempts,
                "storageFailures": counters.storage_failures,
                "pushes": counters.pushes,
                "pushFailures": counters.push_failures,
                "echoesRefused": counters.echoes_refused,
                "echoesAdopted": counters.echoes_adopted,
                "echoesEqual": counters.echoes_equal,
                "echoesAbsent": counters.echoes_absent,
                // Its own key, not folded into `echoesAbsent`: an outcome standing for two is what
                // made `echoesRefused` count the guard never being asked. Expected to stay at zero
                // for this document, which is what makes a non-zero value worth reading.
                "echoesUnparsable": counters.echoes_unparsable,
                "echoesPushedBack": counters.echoes_pushed_back,
                "hostMessagesDropped": super::workspace_groups::native_host_messages_dropped(),
                "prunes": counters.prunes,
                "storageRefusals": counters.storage_refusals,
                "readFailures": counters.read_failures,
                "echoesDeferred": counters.echoes_deferred,
                "deferredRecovered": counters.deferred_recovered,
                "reconcileSeen": counters.reconcile_seen,
                "reconcileEntered": counters.reconcile_entered,
                // Whether the store holds the daemon's copy at all. With `reconcileSeen` this
                // separates the three answers the last two rounds could not tell apart: absent
                // means the document never reached the side state, held with `reconcileSeen` zero
                // means it arrived without ever being reported as a CHANGE, and held with
                // `reconcileSeen` non-zero means the host saw it.
                "sideStateHeld": side_state_held,
            }),
        );
    }

    pub(in crate::app::gx_store) fn sidebar_bulk_ran(
        &mut self,
        request: &ghostex_gx_core::BulkRequest,
        counters: super::sidebar_bulk::SidebarBulkCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarBulk",
            json!({
                "action": log_text(request.action.as_str()),
                "rows": request.messages.len() as u64,
                "intervalMs": request.interval_ms,
                "focusProject": request.focus_project.is_some(),
                "remoteProject": request.is_remote_project(),
                "bulkRequests": counters.bulk_requests,
                "bulkMessages": counters.bulk_messages,
                "pacedRequests": counters.paced_requests,
                "remoteRequests": counters.remote_requests,
                "emptyRequests": counters.empty_requests,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// One line per snooze menu row answered: how many commands it posted, and nothing about the
    /// row. The wake time is NOT recorded; it is a timestamp the user chose for a session of
    /// theirs, and the counters below say everything a support log needs.
    pub(in crate::app::gx_store) fn sidebar_snooze_action_ran(
        &mut self,
        messages: usize,
        counters: super::sidebar_snooze::SidebarSnoozeCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarSnoozeAction",
            json!({
                "messages": messages as u64,
                "actions": counters.actions,
                "actionsWithTag": counters.actions_with_tag,
                "actionsEmpty": counters.actions_empty,
                "declinedSource": counters.declined_source,
                "declinedRow": counters.declined_row,
            }),
        );
    }

    /// One line per snooze or unsnooze call: which one, whether the daemon took it, and how long
    /// it took. No session id and no wake time.
    pub(in crate::app::gx_store) fn sidebar_snooze_ran(
        &mut self,
        call: &str,
        accepted: bool,
        round_trip_ms: u64,
        counters: super::sidebar_snooze::SidebarSnoozeCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarSnooze",
            json!({
                "call": log_text(call),
                "accepted": accepted,
                "roundTripMs": round_trip_ms,
                "snoozes": counters.snoozes,
                "unsnoozes": counters.unsnoozes,
                "acceptedTotal": counters.accepted,
                "failed": counters.failed,
                "sleeps": counters.sleeps,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// One line per dialog the sidebar opened. Which dialog, and whether it was seeded, and
    /// nothing else: the seed IS the user's own title or note.
    pub(in crate::app::gx_store) fn sidebar_modal_opened(
        &mut self,
        is_rename: bool,
        seeded: bool,
        counters: super::sidebar_modals::SidebarModalCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarModal",
            json!({
                "modal": log_text(match is_rename {
                    true => "renameSession",
                    false => "sessionNote",
                }),
                "seeded": seeded,
                "renames": counters.renames,
                "notes": counters.notes,
                "declinedSource": counters.declined_source,
                "declinedRow": counters.declined_row,
            }),
        );
    }

    /// One line per flag call: whether the daemon took it, and how long it took.
    ///
    /// No tag id and no session id. A tag is a short fixed word today, but the catalog is the
    /// user's and a custom tag is text they typed, so it is counted and never written.
    pub(in crate::app::gx_store) fn sidebar_flags_ran(
        &mut self,
        accepted: bool,
        round_trip_ms: u64,
        counters: super::sidebar_flags::SidebarFlagsCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarFlags",
            json!({
                "accepted": accepted,
                "roundTripMs": round_trip_ms,
                "calls": counters.calls,
                "acceptedTotal": counters.accepted,
                "failed": counters.failed,
                "alreadyAgreed": counters.already_agreed,
                "parksThatSleep": counters.parks_that_sleep,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// One line per fork: whether the daemon made one, and how long it took.
    ///
    /// No id and no failure text. The text a fork failure carries is the daemon's or the
    /// transport's and it reaches the user through a toast; it is never written here.
    pub(in crate::app::gx_store) fn sidebar_fork_ran(
        &mut self,
        placed: bool,
        round_trip_ms: u64,
        counters: super::sidebar_lifecycle::SidebarLifecycleCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarFork",
            json!({
                "placed": placed,
                "roundTripMs": round_trip_ms,
                "forks": counters.forks,
                "forksPlaced": counters.forks_placed,
                "forksFailed": counters.forks_failed,
            }),
        );
    }

    /// One line per close: what the daemon said, how long it took, and whether the row came back.
    ///
    /// `closesRestored` is the number to watch. Every one of them is a row the TypeScript would
    /// have left missing for the rest of the run, and a run where it is not zero says the daemon
    /// is refusing or dropping closes, which is worth seeing on its own.
    pub(in crate::app::gx_store) fn sidebar_close_ran(
        &mut self,
        answer: &str,
        round_trip_ms: u64,
        counters: super::sidebar_lifecycle::SidebarLifecycleCounters,
    ) {
        if self.sidebar_lifecycle_records >= MAX_SIDEBAR_ACTION_RECORDS
            || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_lifecycle_records += 1;
        record(
            "gxStore.sidebarClose",
            json!({
                "answer": log_text(answer),
                "roundTripMs": round_trip_ms,
                "closes": counters.closes,
                "closesAccepted": counters.closes_accepted,
                "closesRestored": counters.closes_restored,
                "declinedSource": counters.declined_source,
            }),
        );
    }

    /// One line per sidebar action the store answered: the message type, which calls it made, how
    /// long deciding them took, and the run's totals.
    ///
    /// The record names calls and counts only. The text a copy action carries is a session title,
    /// a project path or a resume command line, and the ids it resolves are the same strings, so
    /// nothing derived from a payload is written beyond its `type`, which is a fixed vocabulary.
    /// `planUs` is the decision, not the call: it is the only part this milestone added to a click
    /// and it is non-zero for the three project-path actions, which resolve the group against the
    /// project facts.
    pub(in crate::app::gx_store) fn sidebar_action_ran(
        &mut self,
        kind: &str,
        plan: &ghostex_gx_core::SidebarActionPlan,
        plan_us: u64,
        counters: super::sidebar_actions::SidebarActionCounters,
    ) {
        if self.sidebar_action_records >= MAX_SIDEBAR_ACTION_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.sidebar_action_records += 1;
        let calls: Vec<serde_json::Value> = plan
            .effects
            .iter()
            .take(SANITIZER_MAX_ENTRIES)
            .map(|effect| {
                serde_json::Value::String(log_text(match effect {
                    ghostex_gx_core::ActionEffect::CopyText { .. } => "copyText".to_string(),
                    // The action name, never the project id the payload carries.
                    ghostex_gx_core::ActionEffect::NativeProjectPathAction { payload } => format!(
                        "nativeProjectPathAction={}",
                        payload
                            .get("action")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("?")
                    ),
                    ghostex_gx_core::ActionEffect::Toast { level, .. } => {
                        format!("toast={}", level.as_str())
                    }
                    // The open family never reaches this line, which is the read-only one's, but
                    // naming the effects here keeps a planner that started emitting one from
                    // being dropped silently. The modal NAME is a fixed word this store builds;
                    // the payload beside it carries project paths and is never named.
                    ghostex_gx_core::ActionEffect::CloseAppModal => "closeAppModal".to_string(),
                    ghostex_gx_core::ActionEffect::OpenAppModal { payload } => format!(
                        "openAppModal={}",
                        payload
                            .get("modal")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("?")
                    ),
                    // Every other effect by its NAME only (a settings patch or a launch message is
                    // the user's). Line-neutral: this file is over its ceiling.
                    other => other.call_name().to_string(),
                }))
            })
            .collect();
        record(
            "gxStore.sidebarAction",
            json!({
                "type": log_text(kind.to_string()),
                "calls": calls,
                "planUs": plan_us,
                "handled": counters.handled,
                "nothing": counters.nothing,
                "copyText": counters.copy_text,
                "nativeProjectPath": counters.native_project_path,
                "toast": counters.toast,
                "declinedSource": counters.declined_source,
            }),
        );
    }
}
