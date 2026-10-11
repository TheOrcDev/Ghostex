//! The drained envelope: what `nativeChat.take(lastRevision)` returns, built from the Rust core.
//!
//! `apps/desktop/src/app/native_chat/state.rs`'s `apply_output` destructures this object key by key
//! (`itemsSplice`, `minimap`, `subagentSplice`, `rowDetails`, `snapshot`, `requests`) and 24,000
//! lines of drawing code read the snapshot by string index. `Frame` already carries the TypeScript
//! spelling of every field, so the envelope is one `to_value`; what this file adds is the effects
//! the view performs and the change gate the drain is posted behind.

use ghostex_gx_chat_core::{Frame, HostRequest};
use serde_json::Value;

/// The object the view applies.
pub(super) fn envelope(mut frame: Frame, requests: Vec<HostRequest>) -> Value {
    frame.requests = requests;
    serde_json::to_value(&frame).unwrap_or(Value::Null)
}

/// Whether a drain is worth posting.
///
/// CDXC:SessionChat 2026-09-22 WHY:
/// All SIX channels, not the four `runtime_worker.rs` tests. `take` has already advanced its sent
/// pointers by the time the gate runs, so a drain whose only change is the minimap or the subagent
/// splice was dropped and never re-sent (`docs/2026-09-21/rust-chat/SEAM.md` section 5.1): the rail
/// stopped updating and an open subagent transcript froze until something else changed.
pub(super) fn envelope_carries_change(frame: &serde_json::Map<String, Value>) -> bool {
    frame.contains_key("itemsSplice")
        || frame.contains_key("minimap")
        || frame.contains_key("subagentSplice")
        || frame.contains_key("rowDetails")
        || frame.contains_key("snapshot")
        || frame
            .get("requests")
            .and_then(Value::as_array)
            .is_some_and(|requests| !requests.is_empty())
}
