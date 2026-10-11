//! The draft-recovery dismissal compaction: per-revision markers folded into ranges.
//!
//! CDXC:Drafts 2026-09-13 WHY:
//! Over 16,000 per-revision dismissal objects filled the browser's storage quota and made ordinary
//! typing fail to save. A dismissed checkpoint is therefore not left as its own record: the markers
//! of one draft are folded into `[start, end]` revision ranges under a single
//! `ghostex.sessionChat.recoveryDismissed.["<sessionKey>","<draftId>"]` record, gaps preserved, and
//! only then are the markers removed. This is the Rust port of `compactDraftRecoveryDismissals` in
//! `packages/core-ui/chat/session-chat-draft-dismissals.ts` (deleted 2026-10-01), writing the same bytes: the same key,
//! `JSON.stringify` of an array of two-element arrays, no spaces.
//!
//! Two rules from the TypeScript are load bearing and are kept in its order:
//!
//! 1. **A legacy marker is shrunk in place first**, before anything else is written, so the
//!    migration still fits when the store is already at its bound. A dismissal written before the
//!    marker form existed is the whole checkpoint object with `dismissed: true`; it is rewritten as
//!    the three-element array when that is shorter.
//! 2. **The ranges are committed before their markers are removed.** A crash between the two leaves
//!    a marker that is already covered by a range, which reads as dismissed twice; the other order
//!    would lose the dismissal entirely and offer the user a draft they threw away.
//!
//! What is NOT ported is the page-wide scan and the write lock around it. The lock is IndexedDB's,
//! against another browser page writing the same ranges; here the one writer is the host thread,
//! and it compacts the session it just retired rather than every key in the store.

use ghostex_gx_chat_core::StorageKey;
use serde_json::Value;

use super::host_records::dismissal_suffix;
use super::storage;

/// One marker: the session it belongs to, the draft, and the dismissed revision.
struct Marker {
    session_key: String,
    draft_id: String,
    revision: i64,
}

/// The markers of one draft, and the record their ranges live in.
struct Group {
    /// The `recoveryDismissed` suffix, which is `["<sessionKey>","<draftId>"]`.
    key: String,
    /// The `recovery` suffixes to remove once the ranges are committed.
    names: Vec<String>,
    revisions: Vec<(i64, i64)>,
}

/// Folds every dismissal marker of one session into its draft's ranges.
///
/// Errors are the storage door's and are returned rather than swallowed, so the caller counts them;
/// a refused compaction leaves the markers where they are and they still read as dismissed.
pub(super) fn compact(session_key: &str, now_ms: i64) -> Result<(), &'static str> {
    let rows = storage::scan("recovery", &format!("{session_key}:"), now_ms)?;
    // Grouped by the record the ranges live in, which is the marker's OWN session key rather than
    // the scanned one: the value is what the TypeScript groups by.
    let mut groups: Vec<Group> = Vec::new();
    for (suffix, raw) in rows {
        let Some(marker) = decode(&raw) else {
            continue;
        };
        let compact = encode(&marker);
        if compact.len() < raw.len() {
            storage::write(
                &StorageKey {
                    store: "recovery".to_string(),
                    suffix: suffix.clone(),
                },
                Some(&compact),
                now_ms,
            )?;
        }
        let group_key = dismissal_suffix(&marker.session_key, &marker.draft_id);
        match groups.iter_mut().find(|group| group.key == group_key) {
            Some(group) => {
                group.names.push(suffix);
                group.revisions.push((marker.revision, marker.revision));
            }
            None => groups.push(Group {
                key: group_key,
                names: vec![suffix],
                revisions: vec![(marker.revision, marker.revision)],
            }),
        }
    }
    for group in groups {
        let key = StorageKey {
            store: "recoveryDismissed".to_string(),
            suffix: group.key,
        };
        let previous: Vec<(i64, i64)> = storage::read(&key, now_ms)?
            .filter(|raw| !raw.is_empty())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        storage::write(&key, Some(&ranges(previous, group.revisions)), now_ms)?;
        for suffix in group.names {
            storage::write(
                &StorageKey {
                    store: "recovery".to_string(),
                    suffix,
                },
                None,
                now_ms,
            )?;
        }
    }
    Ok(())
}

/// `[sessionKey, draftId, revision]`, the marker written over a dismissed checkpoint.
fn encode(marker: &Marker) -> String {
    Value::Array(vec![
        Value::String(marker.session_key.clone()),
        Value::String(marker.draft_id.clone()),
        Value::from(marker.revision),
    ])
    .to_string()
}

/// The marker a stored recovery record holds, in both of its forms, or `None` for a live
/// checkpoint.
///
/// The array is the current form; the object with `dismissed` is what was written before the array
/// existed, and it is read so an installation that has both converges on one.
fn decode(raw: &str) -> Option<Marker> {
    let value: Value = serde_json::from_str(raw).ok()?;
    if let Value::Array(entry) = &value {
        let revision = entry.get(2)?.as_i64()?;
        return Some(Marker {
            session_key: entry.first()?.as_str()?.to_string(),
            draft_id: entry.get(1)?.as_str()?.to_string(),
            revision,
        });
    }
    if value.get("dismissed").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let version = value.get("version")?;
    Some(Marker {
        session_key: value.get("sessionKey")?.as_str()?.to_string(),
        draft_id: version.get("draftId")?.as_str()?.to_string(),
        revision: version.get("revision")?.as_i64()?,
    })
}

/// The stored ranges and the new revisions, merged into the fewest INCLUSIVE `[start, end]` pairs.
///
/// Two ranges join when the second starts at most one past the first's end, so 4 and 5 become
/// `[4, 5]` and a gap at 6 keeps 7 in a range of its own.
fn ranges(previous: Vec<(i64, i64)>, added: Vec<(i64, i64)>) -> String {
    let mut all: Vec<(i64, i64)> = previous.into_iter().chain(added).collect();
    all.sort_by_key(|(start, _)| *start);
    let mut merged: Vec<(i64, i64)> = Vec::with_capacity(all.len());
    for (start, end) in all {
        match merged.last_mut() {
            Some(last) if start <= last.1 + 1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    Value::Array(
        merged
            .into_iter()
            .map(|(start, end)| Value::Array(vec![Value::from(start), Value::from(end)]))
            .collect(),
    )
    .to_string()
}
