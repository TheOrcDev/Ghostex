//! What the phone's React Native composer and card band need from the chat, in one small value.
use serde::Serialize;
use serde_json::Value;

use crate::app::native_chat::state::NativeChatView;

/// The composer-side state of the open chat. Emitted as
/// [`crate::ChatTranscriptEvent::Summary`] whenever it changes, and readable at any time through
/// [`crate::ChatTranscript::summary`]. Serializes to camelCase JSON for a JNI or C bridge.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerSummary {
    /// `projectId:sessionId` of the chat this describes.
    pub session: String,
    /// The core's `status`: `loading`, `ready`, `working`, `error`, `notFound`, `starting`.
    pub status: String,
    /// The agent's turn is live: the composer shows Stop instead of Send.
    pub working: bool,
    /// The chat has read its stored draft and accepts sends ([`crate::ChatTranscript::send_text`]
    /// refuses before this).
    pub composer_ready: bool,
    /// A send left and has not been confirmed or refused yet.
    pub pending_send: bool,
    /// Why a send would be refused right now (a blocking question, a locked conversation), or
    /// `None` when sending is allowed.
    pub send_blocked_reason: Option<String>,
    /// The agent's blocking question or approval card is up (the RN card band draws it from
    /// the snapshot).
    pub question_card_visible: bool,
    /// `question`, `approval` or another kind of the pending prompt, when there is one.
    pub prompt_kind: Option<String>,
    /// The approval card's own question ("Allow this edit?"), empty for a question.
    pub approval_ask: String,
    /// Prompts waiting in Ghostex's queue.
    pub queue_count: usize,
    /// This session can queue a prompt behind a working turn.
    pub can_queue: bool,
    /// The core's collapse state for the composer (reading upward collapses it).
    pub composer_collapsed: bool,
    /// The composer's placeholder, which changes with the prompt and the agent.
    pub placeholder: String,
    /// The composer text as the chat holds it: a restored draft after opening, a failed send put
    /// back, a history recall, a returned prompt. The host shows this text when it changes.
    pub draft: String,
    /// The last action or send error the composer shows under its field.
    pub operation_error: Option<String>,
    /// The chat's own load or connection error.
    pub error: Option<String>,
    /// The transcript has at least one row.
    pub has_rows: bool,
}

impl ComposerSummary {
    pub(crate) fn of(view: &NativeChatView) -> Self {
        let snapshot = &*view.snapshot;
        let text = |value: &Value| value.as_str().map(str::to_string);
        Self {
            session: format!("{}:{}", view.config.project_id, view.config.session_id),
            status: text(&snapshot["status"]).unwrap_or_default(),
            working: snapshot["working"] == true,
            composer_ready: view.composer_ready,
            pending_send: view.pending_send,
            send_blocked_reason: text(&snapshot["sendBlockedReason"]),
            question_card_visible: snapshot["questionCard"]["visible"] == true,
            prompt_kind: text(&snapshot["prompt"]["kind"]),
            approval_ask: text(&snapshot["questionCard"]["approvalAsk"]).unwrap_or_default(),
            queue_count: snapshot["queue"]["prompts"]
                .as_array()
                .map_or(0, |prompts| prompts.len()),
            can_queue: snapshot["queue"]["capabilities"]["canQueue"] == true,
            composer_collapsed: snapshot["composerCollapsed"] == true,
            placeholder: text(&snapshot["composerPlaceholder"]).unwrap_or_default(),
            draft: view.draft.clone(),
            operation_error: text(&snapshot["operationError"]),
            error: view.error.clone().or_else(|| text(&snapshot["error"])),
            has_rows: !view.items.is_empty(),
        }
    }
}
