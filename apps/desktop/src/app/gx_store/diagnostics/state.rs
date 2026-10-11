use std::time::{Duration, Instant};

use serde_json::json;

use super::focus_perform::FocusPerformCounters;
use super::focus_publish::FocusPublishCounters;
use super::sidebar_list::LastUpdate;
use super::sidebar_self_check::SidebarSelfCheckCounters;
use super::sidebar_ui::SidebarUiCounters;
use crate::{shared_settings, support_logs};

/// Records of the kept list disagreeing with a fresh one. Each one is a bug, so a handful is
/// plenty to name it and the counter carries the rate.
pub(super) const MAX_SCRATCH_RECORDS: u32 = 4;
/// Records of a sidebar action. One per click is the whole rate, and the totals ride in each one,
/// so the cap only stops a renderer that repeats a command from filling the log.
pub(in crate::app::gx_store) const MAX_SIDEBAR_ACTION_RECORDS: u32 = 200;
/// Records of an update slow enough to drop a frame. Enough to see whether the spikes are one
/// shape or several; the max in the summary carries the size.
pub(super) const MAX_SLOW_UPDATE_RECORDS: u32 = 8;
/// Comfortably inside the sanitizer's own 120, so a value is cut here where it can say it was cut
/// rather than there where it cannot.
pub(super) const LOG_TEXT_MAX_CHARS: usize = 110;
/// `MAX_SANITIZED_STRING_CHARS` in support_logs.rs, which this must stay under.
pub(super) const SANITIZER_MAX_CHARS: usize = 120;
/// The sanitizer's `take(32)` on every object and array, and its `depth > 4` cap.
pub(super) const SANITIZER_MAX_ENTRIES: usize = 32;
pub(super) const SANITIZER_MAX_DEPTH: usize = 4;
/// Unconditional warning lines one app run may write. A daemon that keeps producing a bad row
/// must not be able to fill the disk through this path.
pub(super) const MAX_WARNING_LINES: u32 = 40;
pub(in crate::app::gx_store) const PERIODIC_SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

/// Log lines of the store, all in the `native.sidebar.refresh` support log.
///
/// Routine lines (`gxStore.loaded`, `gxStore.connection`, `gxStore.focus.summary`) are written only
/// while "Show debug UI controls" and that scenario are on, which the support log enforces.
/// Warnings (a frame that does not parse, snapshot rows that were skipped) are written always,
/// capped per run. Every line holds ids, counts, enum names, and field names: never a title, a
/// path, or frame content.
#[derive(Default)]
pub(crate) struct GxStoreDiagnostics {
    pub(super) warning_lines: u32,
    /// When the summary was last considered, so a run with logging off reads the settings at
    /// most once per interval, and the totals it last wrote.
    pub(super) focus_summary_considered_at: Option<Instant>,
    pub(super) focus_summary_written: (FocusPublishCounters, FocusPerformCounters),
    pub(super) sidebar_summary_considered_at: Option<Instant>,
    pub(super) sidebar_summary_written: SidebarSelfCheckCounters,
    pub(super) sidebar_ui_summary_considered_at: Option<Instant>,
    pub(super) sidebar_ui_summary_written: SidebarUiCounters,
    /// The runtime facts holder's periodic line (`diagnostics_runtime_facts.rs`).
    pub(in crate::app::gx_store) runtime_facts_summary_at: Option<Instant>,
    #[allow(clippy::type_complexity)]
    pub(in crate::app::gx_store) runtime_facts_summary_written: Option<(
        super::runtime_facts::RuntimeFactsCounters,
        super::sidebar_runtime_route::SidebarRuntimeRouteCounters,
    )>,
    /// The budget of `gxStore.sidebarCommandUnroutable`, which repeats for as long as the command
    /// that has no owner keeps being posted.
    pub(in crate::app::gx_store) unroutable_command_warnings: u32,
    pub(super) sidebar_refusal_warnings: u32,
    pub(super) workspace_groups_records: u32,
    pub(super) workspace_groups_summary_at: Option<Instant>,
    pub(super) client_document_records: u32,
    /// The PERIODIC line's own budget.
    ///
    /// CDXC:Projects 2026-09-21 WHY:
    /// The summary emits three records per interval (both documents and the moves) and used to
    /// spend `client_document_records`, so a quiet run of an hour exhausted the two hundred and
    /// then a real push or a real move had no line left to write. The periodic path exists to say
    /// "nothing has happened"; it must not be what silences the path that says something did.
    pub(super) client_document_summary_records: u32,
    pub(super) client_document_read_warnings: u32,
    pub(super) client_document_write_warnings: u32,
    pub(super) client_document_refusal_warnings: u32,
    pub(super) client_document_unparsable_warnings: u32,
    pub(super) client_document_summary_at: Option<Instant>,
    #[allow(clippy::type_complexity)]
    pub(super) client_document_summary_written: Option<(
        super::client_document::ClientDocumentCounters,
        super::client_document::ClientDocumentCounters,
        super::project_docs::ProjectMoveCounters,
    )>,
    pub(super) workspace_groups_summary_written:
        Option<(super::workspace_groups::WorkspaceGroupsCounters, bool)>,
    pub(super) workspace_groups_read_warnings: u32,
    pub(super) workspace_groups_write_warnings: u32,
    pub(super) workspace_groups_refusal_warnings: u32,
    /// The budget of the lines in `diagnostics_project_docs.rs`, apart from
    /// `client_document_records` so a busy launch cannot silence the proof that a Project Group
    /// menu item, a Space editor result or a Space switch ran at all.
    pub(in crate::app::gx_store) project_doc_edit_records: u32,
    pub(super) sidebar_scratch_records: u32,
    pub(super) sidebar_slow_update_records: u32,
    pub(super) sidebar_storage_warnings: u32,
    pub(super) sidebar_action_records: u32,
    pub(in crate::app::gx_store) sidebar_lifecycle_records: u32,
    pub(super) sidebar_drag_records: u32,
    pub(in crate::app::gx_store) remote_last_seen_records: u64,
}

/// What one view-model update was handed, in the shape both records that carry it use.
pub(super) fn last_update_details(last_update: &LastUpdate) -> serde_json::Value {
    json!({
        "changesEmpty": last_update.changes_empty,
        "sessionsChanged": last_update.sessions_changed,
        "sessionsRemoved": last_update.sessions_removed,
        "projectsChanged": last_update.projects_changed,
        "projectsRemoved": last_update.projects_removed,
        "sessionOrderChanged": last_update.session_order_changed,
        "projectOrderChanged": last_update.project_order_changed,
        "machinesReloaded": last_update.machines_reloaded,
        "focusChanged": last_update.focus_changed,
        "workspaceGroups": last_update.workspace_groups,
        "projectCollections": last_update.project_collections,
        "spaces": last_update.spaces,
        "customSessionTags": last_update.custom_session_tags,
        "dirty": last_update.dirty,
        "uiGenerationMoved": last_update.ui_generation_moved,
        "settingsMoved": last_update.settings_moved,
    })
}

pub(in crate::app::gx_store) fn routine_logging_enabled() -> bool {
    shared_settings::shared_sidebar_settings_snapshot().debugging_mode()
        && support_logs::scenario_enabled(support_logs::GpuiDiagnosticScenario::SidebarRefresh)
}

pub(super) fn append(event: &str, details: serde_json::Value) {
    support_logs::append(support_logs::GpuiSupportLog::SidebarRefresh, event, details);
}
