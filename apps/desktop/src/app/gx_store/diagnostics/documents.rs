use std::time::Instant;

use serde_json::json;

use super::*;

impl GxStoreDiagnostics {
    /// A bound refused the workspace session groups document.
    ///
    /// CDXC:Sessions 2026-09-21 WHY:
    /// One budget per condition, not one for all three. They shared `workspace_groups_warnings`
    /// until 2026-09-21, and since the read retries every five seconds a run whose database is
    /// briefly locked burns the whole budget on read failures inside fifteen seconds and then says
    /// nothing at all about a write that is being refused for the rest of the run.
    pub(in crate::app::gx_store) fn workspace_groups_write_refused(&mut self, bound: &'static str) {
        if self.workspace_groups_refusal_warnings >= 3 {
            return;
        }
        self.workspace_groups_refusal_warnings += 1;
        self.warning(
            "gxStore.workspaceGroups.write.refused",
            json!({ "bound": bound }),
        );
    }

    /// A read of the stored document that did not land. Retried; until it does, nothing is adopted
    /// and nothing is edited.
    pub(in crate::app::gx_store) fn workspace_groups_read_failed(&mut self, error: &'static str) {
        if self.workspace_groups_read_warnings >= 3 {
            return;
        }
        self.workspace_groups_read_warnings += 1;
        self.warning(
            "gxStore.workspaceGroups.read.failed",
            json!({ "error": error }),
        );
    }

    /// The stored document could not be read after every fast attempt. Said once, because from here
    /// every edit of the user's groups is refused until a read lands.
    pub(in crate::app::gx_store) fn workspace_groups_read_unavailable(&mut self) {
        self.warning("gxStore.workspaceGroups.read.unavailable", json!({}));
    }

    /// A storage write of that document that did not land. Retried; the warning says it happened.
    pub(in crate::app::gx_store) fn workspace_groups_write_failed(&mut self, error: &'static str) {
        if self.workspace_groups_write_warnings >= 3 {
            return;
        }
        self.workspace_groups_write_warnings += 1;
        self.warning(
            "gxStore.workspaceGroups.write.failed",
            json!({ "error": error }),
        );
    }

    /// A read of a client-owned document's stored key that did not land. Retried; until it does,
    /// nothing is adopted and nothing is edited.
    pub(in crate::app::gx_store) fn client_document_read_failed(
        &mut self,
        name: &'static str,
        error: &'static str,
    ) {
        if self.client_document_read_warnings >= 3 {
            return;
        }
        self.client_document_read_warnings += 1;
        self.warning(
            "gxStore.clientDocument.read.failed",
            json!({ "document": name, "error": error }),
        );
    }

    /// The stored key could not be read after every fast attempt. Said once per document.
    pub(in crate::app::gx_store) fn client_document_read_unavailable(
        &mut self,
        name: &'static str,
    ) {
        self.warning(
            "gxStore.clientDocument.read.unavailable",
            json!({ "document": name }),
        );
    }

    /// A storage write of a client-owned document that did not land. Retried; the warning says so.
    pub(in crate::app::gx_store) fn client_document_write_failed(
        &mut self,
        name: &'static str,
        error: &'static str,
    ) {
        if self.client_document_write_warnings >= 3 {
            return;
        }
        self.client_document_write_warnings += 1;
        self.warning(
            "gxStore.clientDocument.write.failed",
            json!({ "document": name, "error": error }),
        );
    }

    /// A storage bound refused a client-owned document. Its OWN warning budget, because a shared
    /// one let three read failures inside fifteen seconds silence every later refusal.
    pub(in crate::app::gx_store) fn client_document_write_refused(
        &mut self,
        name: &'static str,
        bound: &'static str,
    ) {
        if self.client_document_refusal_warnings >= 3 {
            return;
        }
        self.client_document_refusal_warnings += 1;
        self.warning(
            "gxStore.clientDocument.write.refused",
            json!({ "document": name, "bound": bound }),
        );
    }

    /// An echo that was not a document at all.
    ///
    /// CDXC:Projects 2026-09-21 WHY:
    /// LOUD on purpose. The collections and Spaces documents parse an echo through the wire types
    /// so the guard and the store's own side state cannot disagree about what the daemon said, and
    /// the price is that ONE malformed entry takes the whole echo with it where the TypeScript kept
    /// the others. A user whose collections quietly stopped following the daemon would never find
    /// that cause from the outside, so it is warned as well as counted and carried in every record.
    pub(in crate::app::gx_store) fn client_document_echo_unparsable(&mut self, name: &'static str) {
        if self.client_document_unparsable_warnings >= 3 {
            return;
        }
        self.client_document_unparsable_warnings += 1;
        self.warning(
            "gxStore.clientDocument.echo.unparsable",
            json!({ "document": name }),
        );
    }

    /// One line per client-owned document: what it has done this run, and whether the last push
    /// landed. `ok` is `None` on the periodic path.
    pub(in crate::app::gx_store) fn client_document_record(
        &mut self,
        name: &'static str,
        ok: Option<bool>,
        counters: super::client_document::ClientDocumentCounters,
    ) {
        self.client_document_record_inner(name, ok, counters, false);
    }

    /// `periodic` picks which budget this line spends. The two are separate because the periodic
    /// path writes three lines an interval and would otherwise silence the push and move records.
    fn client_document_record_inner(
        &mut self,
        name: &'static str,
        ok: Option<bool>,
        counters: super::client_document::ClientDocumentCounters,
        periodic: bool,
    ) {
        let budget = match periodic {
            true => &mut self.client_document_summary_records,
            false => &mut self.client_document_records,
        };
        if *budget >= MAX_SIDEBAR_ACTION_RECORDS || !routine_logging_enabled() {
            return;
        }
        *budget += 1;
        record(
            "gxStore.clientDocument",
            json!({
                "document": name,
                "ok": ok,
                "edits": counters.edits,
                "storageWrites": counters.storage_writes,
                "storageRemoves": counters.storage_removes,
                "storageAttempts": counters.storage_attempts,
                "storageFailures": counters.storage_failures,
                "storageRefusals": counters.storage_refusals,
                "pushes": counters.pushes,
                "pushFailures": counters.push_failures,
                "echoesRefused": counters.echoes_refused,
                "echoesAdopted": counters.echoes_adopted,
                "echoesEqual": counters.echoes_equal,
                "echoesAbsent": counters.echoes_absent,
                "echoesUnparsable": counters.echoes_unparsable,
                "echoesPushedBack": counters.echoes_pushed_back,
                "echoesDeferred": counters.echoes_deferred,
                "deferredRecovered": counters.deferred_recovered,
                "readFailures": counters.read_failures,
            }),
        );
    }

    /// The same counters on the periodic path, so a run in which the user moved no project still
    /// says whether the two documents reached the app at all. The first line is emitted with every
    /// counter at zero on purpose: "the path never ran" is the answer that was missing twice.
    pub(in crate::app::gx_store) fn client_document_summary(
        &mut self,
        collections: super::client_document::ClientDocumentCounters,
        spaces: super::client_document::ClientDocumentCounters,
        moves: super::project_docs::ProjectMoveCounters,
    ) {
        if self
            .client_document_summary_written
            .is_some_and(|written| written == (collections, spaces, moves))
            || self
                .client_document_summary_at
                .is_some_and(|at| at.elapsed() < PERIODIC_SUMMARY_INTERVAL)
        {
            return;
        }
        self.client_document_summary_at = Some(Instant::now());
        if !routine_logging_enabled() {
            return;
        }
        self.client_document_summary_written = Some((collections, spaces, moves));
        self.client_document_record_inner("collections", None, collections, true);
        self.client_document_record_inner("spaces", None, spaces, true);
        self.project_move_summary(moves);
    }

    /// What the project moves did this run. Separate from the documents' own line because it is
    /// about the GESTURE, and a run with moves but no document edit is the shape that says the
    /// planner refused every one of them.
    fn project_move_summary(&mut self, counters: super::project_docs::ProjectMoveCounters) {
        if self.client_document_summary_records >= MAX_SIDEBAR_ACTION_RECORDS {
            return;
        }
        self.client_document_summary_records += 1;
        record(
            "gxStore.projectMove",
            json!({
                "moves": counters.moves,
                "refusals": counters.refusals,
                "handOffs": counters.hand_offs,
                "declinedSource": counters.declined_source,
                "collectionEdits": counters.collection_edits,
                "spaceEdits": counters.space_edits,
                "groupOrders": counters.group_orders,
                "renameRequests": counters.rename_requests,
                "spaceEditors": counters.space_editors,
            }),
        );
    }

    /// One line per project move the store answered: what it wrote, and the run's totals.
    pub(in crate::app::gx_store) fn project_move_ran(
        &mut self,
        plan: &ghostex_gx_core::ProjectMovePlan,
        counters: super::project_docs::ProjectMoveCounters,
    ) {
        if self.client_document_records >= MAX_SIDEBAR_ACTION_RECORDS || !routine_logging_enabled()
        {
            return;
        }
        self.client_document_records += 1;
        record(
            "gxStore.projectMove",
            json!({
                "writes": plan.writes.len() as u64,
                "refusal": plan.refusal.unwrap_or("none"),
                "moves": counters.moves,
                "refusals": counters.refusals,
                "handOffs": counters.hand_offs,
                "declinedSource": counters.declined_source,
                "collectionEdits": counters.collection_edits,
                "spaceEdits": counters.space_edits,
                "groupOrders": counters.group_orders,
            }),
        );
    }

    pub(super) fn sidebar_storage_warning(&mut self, event: &'static str, error: &'static str) {
        if self.sidebar_storage_warnings >= 3 {
            return;
        }
        self.sidebar_storage_warnings += 1;
        self.warning(event, json!({ "error": error }));
    }
}
