//! The store's support-log records: the GxStoreDiagnostics state and caps, its store, sidebar,
//! document and action records, and the record gate that checks every line against the sanitizer.

// The moved bodies name their gx_store siblings as `super::<module>`; importing those modules here
// keeps every such path resolving unchanged from the per-concern files below.
use super::{
    client_document, focus_perform, focus_publish, host, project_docs, remote_clients,
    runtime_facts, sidebar_actions, sidebar_bulk, sidebar_drag, sidebar_flags, sidebar_lifecycle,
    sidebar_list, sidebar_modals, sidebar_runtime_route, sidebar_scratch_compare,
    sidebar_self_check, sidebar_snapshot, sidebar_snooze, sidebar_ui, workspace_groups,
};

pub(crate) mod action_records;
pub(crate) mod documents;
pub(crate) mod record;
pub(crate) mod sidebar_summaries;
pub(crate) mod state;
pub(crate) mod store_events;

pub(super) use record::*;
pub(crate) use state::*;
