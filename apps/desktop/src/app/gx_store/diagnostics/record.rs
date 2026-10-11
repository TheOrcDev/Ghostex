use ghostex_gx_core::{Core, MachineId};
use serde_json::json;

use super::*;

/// Appends a record after checking it against every rule the log's sanitizer applies.
///
/// CDXC:Sidebar 2026-09-20 WHY:
/// `sanitize_json_value` (support_logs.rs:463) has THREE rules, and this milestone shipped a
/// diagnostic that failed each of them in turn: values below depth 4 become "[depth-capped]",
/// strings over 120 characters or holding a slash become "[redacted]", and an object or array
/// keeps only its first 32 entries and drops the rest in silence. The third is the worst of them
/// because nothing in the output says anything was lost: this summary reached 41 keys over several
/// rounds and quietly stopped reporting the nine timings at the end of it. Checking a record by
/// eye is how all three got through, so it is checked here instead, on the way out, in every debug
/// build.
pub(in crate::app::gx_store) fn record(event: &'static str, details: serde_json::Value) {
    debug_assert_loggable(&details, 0, event);
    append(event, details);
}

#[cfg(debug_assertions)]
fn debug_assert_loggable(value: &serde_json::Value, depth: usize, event: &str) {
    match value {
        serde_json::Value::String(text) => {
            debug_assert!(
                depth <= SANITIZER_MAX_DEPTH,
                "{event}: a string at depth {depth} is capped"
            );
            debug_assert!(
                text.chars().count() <= SANITIZER_MAX_CHARS
                    && !text.contains('/')
                    && !text.contains('\\')
                    && !text.chars().any(char::is_control),
                "{event}: a string would be redacted"
            );
        }
        serde_json::Value::Array(items) => {
            debug_assert!(
                items.len() <= SANITIZER_MAX_ENTRIES,
                "{event}: an array of {} drops entries past {SANITIZER_MAX_ENTRIES}",
                items.len()
            );
            for item in items {
                debug_assert_loggable(item, depth + 1, event);
            }
        }
        serde_json::Value::Object(entries) => {
            debug_assert!(
                entries.len() <= SANITIZER_MAX_ENTRIES,
                "{event}: an object of {} keys drops the ones past {SANITIZER_MAX_ENTRIES}",
                entries.len()
            );
            for (key, item) in entries {
                debug_assert!(
                    key.chars().count() <= SANITIZER_MAX_CHARS,
                    "{event}: a key would be redacted"
                );
                debug_assert_loggable(item, depth + 1, event);
            }
        }
        other => debug_assert!(
            depth <= SANITIZER_MAX_DEPTH,
            "{event}: {other} sits at depth {depth}, which is capped"
        ),
    }
}

#[cfg(not(debug_assertions))]
fn debug_assert_loggable(_value: &serde_json::Value, _depth: usize, _event: &str) {}

/// A string the log's sanitizer will print rather than replace.
///
/// CDXC:Sidebar 2026-09-20 WHY:
/// `sanitize_string_value` (support_logs.rs:492) replaces a value with `[redacted]` when it runs
/// past 120 characters or contains a slash, a backslash or a control character, and it does that
/// to object KEYS as well as values. Three diagnostics in this milestone have failed at the moment
/// they were needed, twice for the depth cap and once here, so every string these records emit
/// goes through this: over-long values are cut with a marker instead of vanishing, and a slash is
/// replaced rather than taking the whole value with it. Nothing this writes is private: these
/// records carry ids, field names and counts, and the length rule is about paths, which none of
/// them are.
///
/// The depth rule is the other half and is not something a helper can enforce: a value must sit at
/// depth 4 or less, counting `details` as 0. An array of short strings under `details` is depth 2,
/// and `details.<key>[i].<k>[j]` is depth 4, which is the deepest shape any record here uses.
pub(in crate::app::gx_store) fn log_text(value: impl Into<String>) -> String {
    let value: String = value.into();
    let value = value.replace(['/', '\\'], "|");
    let value: String = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let value = if value.chars().count() <= LOG_TEXT_MAX_CHARS {
        value
    } else {
        let kept: String = value.chars().take(LOG_TEXT_MAX_CHARS - 3).collect();
        format!("{kept}...")
    };
    // The rules this has to satisfy are `sanitize_string_value`'s, and a value that fails them is
    // not logged, it is replaced by a word that says nothing. A debug build says so at the point
    // the value is built rather than leaving it to be discovered in a support log months later.
    debug_assert!(
        value.chars().count() <= SANITIZER_MAX_CHARS
            && !value.contains('/')
            && !value.contains('\\')
            && !value.chars().any(char::is_control),
        "a log value must survive the sanitizer"
    );
    value
}

/// `[{id, fields:[...]}]`: the id at depth 3 and each field name at depth 4, both inside the
/// sanitizer's length rule because a field name is one short word.
pub(super) fn named_fields(entries: &[(String, Vec<String>)]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|(id, fields)| json!({ "id": log_text(id.as_str()), "fields": log_texts(fields) }))
        .collect()
}

pub(super) fn log_texts<'a>(
    values: impl IntoIterator<Item = &'a String>,
) -> Vec<serde_json::Value> {
    values
        .into_iter()
        .map(|value| serde_json::Value::String(log_text(value.as_str())))
        .collect()
}

pub(super) fn store_revision(core: &Core) -> Option<i64> {
    core.presentation()
        .loaded(&MachineId::Local)
        .map(|loaded| loaded.revision)
}
