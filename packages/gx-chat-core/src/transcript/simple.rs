//! The one-line labels simple mode puts on a folded group.
//!
//! Ported from `packages/shared/session-chat-presentation/simple.ts`.

/// "Edited 3 files", the label a folded turn's file changes carry.
pub fn simple_edit_label(count: usize) -> String {
    format!(
        "Edited {count} {}",
        if count == 1 { "file" } else { "files" }
    )
}

/// "3 tool calls", or "Tool output" when the group has no call of its own.
pub fn tool_count_label(count: usize) -> String {
    if count == 0 {
        "Tool output".to_string()
    } else {
        format!("{count} tool {}", if count == 1 { "call" } else { "calls" })
    }
}
