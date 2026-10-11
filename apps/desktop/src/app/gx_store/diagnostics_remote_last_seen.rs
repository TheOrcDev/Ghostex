//! The record lines for the last-seen view of an offline remote machine, in a sibling because
//! `diagnostics.rs` is over the size ceiling and waiting for a quiet window.
//!
//! CDXC:RemoteMachines 2026-09-21 WHY:
//! The payload these lines are about holds session TITLES and project PATHS, so none of it reaches
//! a record: a seed line carries the machine count and the byte size and nothing else, and it does
//! not name the machine either, because a machine id is a short fixed string but the size and the
//! count are all a support log needs to say whether the copy was read. A failure carries the error
//! WORD from the door's own closed vocabulary, never a path and never the payload.
//!
//! SEE-ALSO: apps/desktop/src/app/gx_store/remote_last_seen.rs,
//! apps/desktop/src/app/gx_store/remote_clients.rs.

use serde_json::json;

use super::diagnostics::{GxStoreDiagnostics, log_text, record, routine_logging_enabled};
use super::remote_clients::RemoteClientCounters;
use super::remote_last_seen::RemoteLastSeenCounters;

/// One line per seed, plus the run's totals. Bounded like the other per-event records: there is
/// one seed per machine per run, so a handful of lines at most, and the cap is the machine list.
const MAX_LAST_SEEN_RECORDS: u64 = 32;

impl GxStoreDiagnostics {
    /// A machine's last-seen rows reached the store, so its tab draws faded instead of empty.
    pub(super) fn remote_last_seen_seeded(&mut self, bytes: usize, counters: RemoteClientCounters) {
        if self.remote_last_seen_records >= MAX_LAST_SEEN_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.remote_last_seen_records += 1;
        record(
            "gxStore.remoteLastSeen",
            json!({
                "bytes": bytes as u64,
                "seeds": counters.last_seen_seeds,
                "absent": counters.last_seen_absent,
                "expired": counters.last_seen_expired,
                "failures": counters.last_seen_failures,
            }),
        );
    }

    /// The copy could not be read, written, or parsed. Not a warning: a machine with no usable
    /// copy draws an empty tab, which is what every build before this one drew.
    pub(super) fn remote_last_seen_failed(&mut self, code: &str) {
        if self.remote_last_seen_records >= MAX_LAST_SEEN_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.remote_last_seen_records += 1;
        record(
            "gxStore.remoteLastSeen.failed",
            json!({ "code": log_text(code) }),
        );
    }

    /// A storage bound refused the copy. Its own line rather than a failure, because a refusal is
    /// permanent for that payload and a failure is a lock race that retries.
    pub(super) fn remote_last_seen_refused(&mut self, bound: &str) {
        if self.remote_last_seen_records >= MAX_LAST_SEEN_RECORDS || !routine_logging_enabled() {
            return;
        }
        self.remote_last_seen_records += 1;
        record(
            "gxStore.remoteLastSeen.refused",
            json!({ "bound": log_text(bound) }),
        );
    }

    /// Both halves' counters on the periodic path, whether or not anything has happened.
    ///
    /// CDXC:RemoteMachines 2026-09-21 WHY:
    /// The seed line is emitted from a seed and the write lines from a write, so a run with no
    /// remote machine offline and no remote machine connected has neither, and "the path never
    /// ran" is then indistinguishable from "the scenario was off". The counters therefore ride the
    /// same periodic path as `gxStore.sidebarShadow.summary`, which is proved to reach the log in a
    /// quiet run, and the first line goes out with every counter at zero. Read it as a pair:
    /// `dirtied` above zero with `attempts` at zero means the debounce never fired, and `writes` at
    /// zero with `unchanged` climbing means the sidebar page's copy was already exactly this one,
    /// which is what makes two writers of this key safe.
    ///
    /// The throttle state lives on the writer rather than beside the other summaries', because
    /// `diagnostics.rs` is over the size ceiling and waiting for a quiet window.
    pub(super) fn remote_last_seen_summary(
        &mut self,
        read: RemoteClientCounters,
        write: RemoteLastSeenCounters,
    ) {
        if !routine_logging_enabled() {
            return;
        }
        record(
            "gxStore.remoteLastSeen.summary",
            json!({
                "seeds": read.last_seen_seeds,
                "absent": read.last_seen_absent,
                "expired": read.last_seen_expired,
                "readFailures": read.last_seen_failures,
                "dirtied": write.dirtied,
                "attempts": write.attempts,
                "writes": write.writes,
                "unchanged": write.unchanged,
                "refusals": write.refusals,
                "failures": write.failures,
                "largestBytes": write.largest_bytes,
                "buildMaxUs": write.build_max_us,
            }),
        );
    }
}
