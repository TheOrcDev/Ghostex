//! Which card a system turn renders as.
//!
//! Ported from `packages/shared/session-chat-presentation/system-cards.ts`. Both renderers classify
//! here so a row cannot be a titled card in one and raw prose in the other; each renderer owns only
//! the layout of the card it is handed.

use ghostex_gx_protocol::{ChatBlock, ChatMessage, ChatRole};
use serde_json::{json, Value};

use crate::transcript::agent_message::{agent_display_name, parse_agent_message};
use crate::transcript::foreign::{CODEX_GOAL_ID_PREFIX, FORK_BOUNDARY_ID_PREFIX};
use crate::transcript::jsstr::js_trim;

/// The id `sessionChatPendingMessages` writes for a local command's captured output.
const APP_COMMAND_OUTPUT_ID_PREFIX: &str = "app-command-output:";
/// The id it writes for a command Ghostex ran on the session's behalf, of which the rename is the
/// one with its own card.
const APP_COMMAND_ID_PREFIX: &str = "app-command:";
pub(crate) const AUTO_NAMED_TITLE_LEAD: &str = "Ghostex auto named this session";
/// CDXC:SessionTitles 2026-09-30 WHY: a name the user typed (sidebar Rename, rename modal) reaches the agent as the same `/rename` Ghostex sends for its own titles, so gxserver marks it `userRename` and the card says so instead of claiming Ghostex named the session.
pub(crate) const USER_RENAMED_TITLE_LEAD: &str = "Session renamed";

fn block_text(message: &ChatMessage, index: usize) -> &str {
    match message.blocks.get(index) {
        Some(ChatBlock::Text { text }) => text,
        _ => "",
    }
}

/// Which card a system turn renders as, or `Value::Null` when the turn is not a system turn.
///
/// Suppressed harness turns are decided first by the noise classifier and never reach this.
/// `marker` is the ordinary case: one muted line of the text the daemon wrote.
pub fn classify_system_card(message: &ChatMessage, markdown: &str) -> Value {
    if message.role != ChatRole::System {
        return Value::Null;
    }
    let lead = block_text(message, 0);
    let user_renamed = lead == USER_RENAMED_TITLE_LEAD;
    let renamed_title = if message.id.starts_with(APP_COMMAND_ID_PREFIX)
        && (lead == AUTO_NAMED_TITLE_LEAD || user_renamed)
    {
        js_trim(block_text(message, 1))
    } else {
        ""
    };
    if !renamed_title.is_empty() {
        return json!({
            "kind": "auto-named",
            "title": renamed_title,
            "lead": lead,
            "userRenamed": user_renamed,
        });
    }
    if message.id.starts_with(FORK_BOUNDARY_ID_PREFIX) {
        return json!({ "kind": "fork-boundary", "text": markdown });
    }
    if message.id.starts_with(CODEX_GOAL_ID_PREFIX) {
        return json!({
            "kind": "goal",
            "status": block_text(message, 0),
            "objective": block_text(message, 1),
            "usage": block_text(message, 2),
        });
    }
    if message.id.starts_with(APP_COMMAND_OUTPUT_ID_PREFIX) {
        return json!({
            "kind": "command-output",
            "command": block_text(message, 0),
            "output": block_text(message, 1),
        });
    }
    if let Some(agent_message) = parse_agent_message(markdown) {
        return json!({
            "kind": "agent-message",
            "sender": agent_message.sender,
            "name": agent_display_name(&agent_message.sender),
            "body": agent_message.body,
        });
    }
    json!({ "kind": "marker", "text": markdown })
}
