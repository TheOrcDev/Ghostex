use std::time::Instant;

use serde_json::json;

use super::sidebar_list::{LastUpdate, SidebarListCounters};
use super::sidebar_scratch_compare::ScratchDifference;
use super::sidebar_self_check::SidebarSelfCheckCounters;
use super::sidebar_ui::SidebarUiCounters;
use super::*;

impl GxStoreDiagnostics {
    /// The running totals of the sidebar list and its self check, at most once a minute and only
    /// when they moved.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::app::gx_store) fn sidebar_summary(
        &mut self,
        counters: &SidebarSelfCheckCounters,
        list: &SidebarListCounters,
        ready: bool,
        // Which of the two legs the list waits for was given up on and drawn without
        // (gx_store/sidebar_ready.rs). Both false on an ordinary run.
        recovered: (bool, bool),
        deadline_kind: &'static str,
        groups: usize,
        rows: usize,
        phases: super::sidebar_snapshot::InstallPhases,
        remote: &super::remote_clients::RemoteClientCounters,
        machines: usize,
    ) {
        if *counters == self.sidebar_summary_written
            || self
                .sidebar_summary_considered_at
                .is_some_and(|at| at.elapsed() < PERIODIC_SUMMARY_INTERVAL)
        {
            return;
        }
        self.sidebar_summary_considered_at = Some(Instant::now());
        if !routine_logging_enabled() {
            return;
        }
        self.sidebar_summary_written = *counters;
        record(
            "gxStore.sidebarList.summary",
            json!({
                // The list is the only one there is; this says whether it is the REAL one or the
                // loading skeleton the launch window draws.
                "ready": ready,
                // Ready only because a leg was declared absent, which the `.error` line names.
                "recoveredHud": recovered.0,
                "recoveredState": recovered.1,
                "storeGroups": groups,
                "storeRows": rows,
                "deadlineKind": deadline_kind,
                // Grouped rather than flat: the sanitizer keeps the first 32 keys of an object and
                // drops the rest without saying so, and this record passed 32 as it grew.
                "scratch": {
                    "checks": counters.scratch_checks,
                    "mismatches": counters.scratch_mismatches,
                },
                // One client per connected remote machine. `machines` is how many tabs the sidebar
                // offers, `starts` how many clients this run opened, and `unloads` how many
                // machines lost their rows because the user disabled or removed them.
                "remote": {
                    "machines": machines,
                    "starts": remote.starts,
                    "stops": remote.stops,
                    "unloads": remote.unloads,
                    "threadExits": remote.thread_exits,
                    "events": remote.events,
                    "reloads": remote.reloads,
                },
                "list": {
                    "updates": list.updates,
                    "updatesIdle": list.idle,
                    "viewChanges": list.view_changes,
                    "installs": list.installs,
                    "installsSkipped": list.installs_skipped,
                    "installsFromCarry": list.installs_from_carry,
                    "loadingInstalls": list.loading_installs,
                    "deadlineWakes": list.deadline_wakes,
                    "wakeRowsMoved": list.wake_rows_moved,
                    "wakeRowsMovedMax": list.wake_rows_moved_max,
                },
                "timings": {
                    "updateUs": list.last_update_us,
                    "updateMaxUs": list.update_max_us,
                    "installUs": list.last_install_us,
                    "installMaxUs": list.install_max_us,
                    "relabelUs": list.last_relabel_us,
                    "relabelMaxUs": list.relabel_max_us,
                },
                // Where the newest install's time went, and what the caches saved it. A rise in
                // installUs says which part of the build it came from rather than inviting a
                // guess: the shared key, the rows, the group menus, the collection menus, the more
                // menu, or the tail that copies what the view model does not own.
                "install": {
                    "hostUs": phases.host_us,
                    "keyUs": phases.key_us,
                    "rowsUs": phases.rows_us,
                    "groupsUs": phases.groups_us,
                    "collectionsUs": phases.collections_us,
                    "moreMenuUs": phases.more_menu_us,
                    "tailUs": phases.tail_us,
                    "rowsBuilt": phases.rows_built,
                    "rowsReused": phases.rows_reused,
                    "groupsBuilt": phases.groups_built,
                    "groupsReused": phases.groups_reused,
                    "collectionsBuilt": phases.collections_built,
                    "collectionsReused": phases.collections_reused,
                    "moreMenuBuilt": phases.more_menu_built,
                    // Why the group phase cost what it did: the shared key missing drops every
                    // cached menu at once, a moved `GroupCore` is the view model having rebuilt
                    // that group, and a moved collection is only the folder it is drawn in.
                    "keyReused": phases.key_reused,
                    "groupsMissing": phases.groups_missing,
                    "groupsCoreMoved": phases.groups_core_moved,
                    "groupsCollectionMoved": phases.groups_collection_moved,
                },
            }),
        );
    }

    /// One record per slow view-model update: what it was handed and what it rebuilt.
    ///
    /// CDXC:Sidebar 2026-09-20 WHY:
    /// `updateMaxUs` is one number with no story: a twenty-millisecond update and a twenty-
    /// microsecond one are the same call from outside, and the difference is always which caches
    /// the update had to drop. Guessing from the median is how a spike gets attributed to whatever
    /// landed in the same release. The counts here answer it outright: `rowsAllDirty` with a row
    /// count is the tag catalog or Debugging Mode moving, `reset` is a machine (un)loading,
    /// `meta` is the project facts, and all three false with a large `rowsBuilt` is the store
    /// having really changed that many rows. `offCpuUs` near `updateUs` with all of them small is
    /// none of those: the thread was not running, and the update is a victim rather than a cause.
    pub(in crate::app::gx_store) fn sidebar_slow_update(
        &mut self,
        update_us: u64,
        cpu_us: Option<u64>,
        last_update: &LastUpdate,
        work: &ghostex_gx_core::SidebarUpdateWork,
    ) {
        if self.sidebar_slow_update_records >= MAX_SLOW_UPDATE_RECORDS || !routine_logging_enabled()
        {
            return;
        }
        self.sidebar_slow_update_records += 1;
        record(
            "gxStore.sidebarList.slowUpdate",
            json!({
                "updateUs": update_us,
                // What the thread actually spent running, and what it spent not running. Nothing
                // inside the update takes a lock or touches the file system, so a large gap is the
                // thread descheduled or in the kernel rather than this code being slow.
                "cpuUs": cpu_us,
                "offCpuUs": cpu_us.map(|cpu| update_us.saturating_sub(cpu)),
                "work": {
                    "reset": work.reset,
                    "rowsAllDirty": work.rows_all_dirty,
                    "metaDirty": work.meta_dirty,
                    "rowsBuilt": work.rows_built,
                    "browserRowsBuilt": work.browser_rows_built,
                    "groupsBuilt": work.groups_built,
                    "machineSummariesBuilt": work.machine_summaries_built,
                    "groups": work.group_count,
                    "rows": work.row_count,
                },
                "lastUpdate": last_update_details(last_update),
            }),
        );
    }

    /// One record per distinct difference between the kept list and one built from nothing.
    ///
    /// CDXC:Sidebar 2026-09-20 WHY:
    /// This is the record for a cache this port failed to invalidate, which is a different animal
    /// from a difference with the old projection: both sides here are this code reading the same
    /// store at the same instant, so one of them is simply wrong. It carries what the newest
    /// update was handed as well as the difference, because the answer is always "which input
    /// moved without the thing that depends on it being dropped", and the update's flags are where
    /// that starts.
    pub(in crate::app::gx_store) fn sidebar_scratch_mismatch(
        &mut self,
        difference: &ScratchDifference,
        last_update: &LastUpdate,
    ) {
        if self.sidebar_scratch_records >= MAX_SCRATCH_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.sidebar_scratch_records += 1;
        record(
            "gxStore.sidebarShadow.scratchMismatch",
            json!({
                "onlyIncrementalGroups": log_texts(&difference.only_incremental_groups),
                "onlyScratchGroups": log_texts(&difference.only_scratch_groups),
                "groupOrderDiffers": difference.group_order_differs,
                "topLevel": log_texts(&difference.top_level),
                "groups": named_fields(&difference.groups),
                "rows": named_fields(&difference.rows),
                "onlyIncrementalRows": log_texts(&difference.only_incremental_rows),
                "onlyScratchRows": log_texts(&difference.only_scratch_rows),
                "lastUpdate": last_update_details(last_update),
            }),
        );
    }

    /// The running totals of the sidebar's own state and its writes, on the same schedule.
    pub(in crate::app::gx_store) fn sidebar_ui_summary(&mut self, counters: &SidebarUiCounters) {
        if *counters == self.sidebar_ui_summary_written
            || self
                .sidebar_ui_summary_considered_at
                .is_some_and(|at| at.elapsed() < PERIODIC_SUMMARY_INTERVAL)
        {
            return;
        }
        self.sidebar_ui_summary_considered_at = Some(Instant::now());
        if !routine_logging_enabled() {
            return;
        }
        self.sidebar_ui_summary_written = *counters;
        record(
            "gxStore.sidebarUi.summary",
            json!({
                "intents": counters.intents,
                "writes": counters.writes,
                "writeFailures": counters.write_failures,
                "readFailures": counters.read_failures,
                "writeRefusals": counters.write_refusals,
                "writeMaxUs": counters.write_max_us,
                "writeOpenMaxUs": counters.write_open_max_us,
                "writeTotalsMaxUs": counters.write_totals_max_us,
                "writeBeginMaxUs": counters.write_begin_max_us,
                "writeStoredMaxUs": counters.write_stored_max_us,
                "writeCommitMaxUs": counters.write_commit_max_us,
                "writeCallMaxUs": counters.write_call_max_us,
            }),
        );
    }

    /// The sidebar's own state could not be read from client storage, so the Rust list is not
    /// drawn and nothing is written. The code is a fixed word: a database error string can carry
    /// the file's path.
    pub(in crate::app::gx_store) fn sidebar_ui_read_failed(&mut self, error: &'static str) {
        self.sidebar_storage_warning("gxStore.sidebarUi.read.warning", error);
    }

    /// A write of the sidebar's own state did not reach client storage. The change is still held
    /// in memory and is written again with the next one.
    pub(in crate::app::gx_store) fn sidebar_ui_write_failed(&mut self, error: &'static str) {
        self.sidebar_storage_warning("gxStore.sidebarUi.write.warning", error);
    }

    /// A storage bound refused one value. Counted apart from a failure, and with its own budget of
    /// lines, because a refusal repeats for as long as the payload stays that size and would
    /// otherwise use up the warnings a real failure needs.
    pub(in crate::app::gx_store) fn sidebar_ui_write_refused(
        &mut self,
        key: &'static str,
        bound: &'static str,
    ) {
        if self.sidebar_refusal_warnings >= 3 {
            return;
        }
        self.sidebar_refusal_warnings += 1;
        self.warning(
            "gxStore.sidebarUi.write.refused",
            json!({ "key": key, "bound": bound }),
        );
    }

    /// The sidebar's own state could not be read after every fast attempt. Said once, as a
    /// warning rather than a routine line, because from here nothing the user collapses, hides or
    /// filters will survive a restart until a read lands.
    pub(in crate::app::gx_store) fn sidebar_ui_read_unavailable(&mut self) {
        self.warning("gxStore.sidebarUi.read.unavailable", json!({}));
    }
}
