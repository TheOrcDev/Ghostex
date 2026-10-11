//! Which picture the transcript region draws, and how a state frame may move the status.
//!
//! Ported from `packages/core-ui/chat/session-chat-view-state.ts`.

use ghostex_gx_protocol::ChatStatus;

use crate::document::ViewState;

/// CDXC:SessionChat 2026-09-10 DECISION:
/// User: separate transcript readiness from agent activity; draft saves, model updates, and
/// working/idle changes must not turn an initialized chat back into loading.
/// Draft autosaves and activity updates carry ready/working without reading the transcript.
/// Letting those frames replace starting made G42qx and G43qg remove an input the user was already
/// typing into.
/// Only a transcript read, snapshot, or appended messages can establish transcript readiness;
/// loading belongs to initial session setup.
pub fn transcript_status_after_state(current: ChatStatus, incoming: ChatStatus) -> ChatStatus {
    match incoming {
        ChatStatus::Working | ChatStatus::Ready | ChatStatus::Loading => current,
        other => other,
    }
}

/// CDXC:SessionChat 2026-09-10 WHY:
/// A Codex session ID can exist before any transcript, including for an empty session launched
/// from Terminal without Ghostex's draft marker.
/// Working with zero messages is an empty conversation, not a new loading phase that may unmount
/// its composer.
/// This supersedes the draft-only exception to the working-without-messages loading hold.
pub fn select_view_state(
    status: &ChatStatus,
    message_count: usize,
    error: Option<&str>,
) -> ViewState {
    if matches!(status, ChatStatus::Error) {
        return ViewState {
            kind: "error".to_string(),
            is_working: ghostex_gx_protocol::Tri::Absent,
            error: ghostex_gx_protocol::Tri::Value(
                error
                    .unwrap_or("Conversation could not be loaded.")
                    .to_string(),
            ),
        };
    }
    if message_count > 0 {
        return ViewState {
            kind: "ready".to_string(),
            is_working: ghostex_gx_protocol::Tri::Value(matches!(status, ChatStatus::Working)),
            error: ghostex_gx_protocol::Tri::Absent,
        };
    }
    let kind = match status {
        ChatStatus::Unsupported => "unsupported",
        ChatStatus::Starting => "starting",
        ChatStatus::Loading => "loading",
        // Empty wins over a transient 'working' so a just-toggled pre-session pane shows a clear
        // empty state instead of a spinner over nothing.
        _ => "empty",
    };
    ViewState {
        kind: kind.to_string(),
        is_working: ghostex_gx_protocol::Tri::Absent,
        error: ghostex_gx_protocol::Tri::Absent,
    }
}
