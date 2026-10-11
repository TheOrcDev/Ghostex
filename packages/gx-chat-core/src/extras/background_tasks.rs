//! The rows a `shells-running` activity draws: one per background task the agent left running.
//!
//! CDXC:SessionChat 2026-10-11 DECISION:
//! User: "1 shell still running" was not clear enough for non-technical users. From the mockups they picked "M3" for every count, also one task: a muted line "Still running in the background · the agent finished its reply at 2:37 AM", then one row per task with its own timer, up to three rows and then "+N more". A row is titled by the description the agent gave the command, a monitor reads "Watching …", and the command itself shows on hover (desktop) or long-press (phone). When a task ends, the chat's existing finished row stays exactly as it looks and only gains how long the task ran (pick "F2"; see `transcript::noise::task_duration_label`).
//!
//! CDXC:SessionChat 2026-10-11 SEE-ALSO:
//! server/src/session_chat_claude_background.rs reads the tasks from Claude's transcript; apps/desktop/src/app/native_chat/working_strip.rs and apps/mobile/app/src/chat/native/cards/WorkingStrip.tsx draw these rows and must match.

use serde_json::{json, Value};

use crate::extras::activity::format_activity_elapsed;
use crate::extras::time::parse_iso_millis;

/// Rows shown before the rest fold into "+N more".
pub const MAX_BACKGROUND_TASK_ROWS: usize = 3;

/// Whether the activity names background tasks whose clocks move.
pub fn has_background_task_clock(activity: &Value) -> bool {
    activity
        .pointer("/background/tasks")
        .and_then(Value::as_array)
        .is_some_and(|tasks| !tasks.is_empty())
}

/// `{ header, rows: [{ id, prefix, title, command, elapsed }], more }` for the strip, or `None` when
/// the activity carries no `background` (another kind, or a gxserver that predates the field, whose
/// strip keeps the footer's own label).
pub fn background_tasks(activity: &Value, now_ms: f64, utc_offset_minutes: i32) -> Option<Value> {
    let background = activity.get("background")?.as_object()?;
    let tasks: Vec<&Value> = background
        .get("tasks")
        .and_then(Value::as_array)
        .map(|tasks| tasks.iter().collect())
        .unwrap_or_default();
    let count = background
        .get("count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .max(tasks.len() as u64);
    let header = match background.get("replyDoneAt").and_then(Value::as_str) {
        Some(done) if !done.trim().is_empty() => format!(
            "Still running in the background \u{b7} the agent finished its reply at {}",
            done.trim()
        ),
        _ => "Still running in the background".to_string(),
    };
    let rows: Vec<Value> = tasks
        .iter()
        .take(MAX_BACKGROUND_TASK_ROWS)
        .map(|task| {
            let started = task
                .get("startedAt")
                .and_then(Value::as_str)
                .and_then(|at| parse_iso_millis(at, utc_offset_minutes));
            json!({
                "id": task.get("id").cloned().unwrap_or(Value::Null),
                "prefix": (task.get("kind").and_then(Value::as_str) == Some("monitor"))
                    .then_some("Watching"),
                "title": task.get("title").and_then(Value::as_str).unwrap_or_default(),
                "command": task.get("command").and_then(Value::as_str),
                "elapsed": started
                    .map(|started| format_activity_elapsed(((now_ms - started) / 1_000.0).max(0.0))),
            })
        })
        .collect();
    let hidden = count.saturating_sub(rows.len() as u64);
    let more = match (rows.is_empty(), hidden) {
        (_, 0) => None,
        // Nothing named: the transcript did not record what was started, so only the count is known.
        (true, 1) => Some("1 command".to_string()),
        (true, n) => Some(format!("{n} commands")),
        (false, n) => Some(format!("+{n} more")),
    };
    Some(json!({ "header": header, "rows": rows, "more": more }))
}
