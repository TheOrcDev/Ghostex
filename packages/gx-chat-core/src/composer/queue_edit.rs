//! Edit on a queued row: take it out of the queue and put its text in the composer.
//!
//! `editQueuedChatPrompt` in `packages/shared/session-chat-controller/submission.ts`, driven by
//! the `removeQueue` action with `edit: true` in `native-host.ts`: remove the row, read what the
//! composer holds right now (`rpc('readNativeComposer')`, which the host answers with the field's
//! text), queue that text when it is not blank so the edit does not throw it away, then insert the
//! removed row's text. Three awaits, so three answers; [`advance`] walks them one at a time.

use serde_json::{json, Value};

use crate::composer::queue::queue_capabilities;
use crate::effect::Effect;
use crate::state::ChatState;
use crate::wire::{ChatRpcMethod, RpcOutcome};

/// The step an edit is waiting on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueEditStage {
    /// `await options.remove()`.
    Removing,
    /// `await options.readCurrent()`.
    Reading,
    /// `await options.queue(current)`.
    Queueing,
}

/// An edit in flight: the request it waits on, and the text it will put in the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueEdit {
    pub request_id: u64,
    pub stage: QueueEditStage,
    /// The row's text as the strip showed it, replaced by the removed row's own text once the
    /// removal answers (`removed?.text ?? options.original`).
    pub text: String,
}

/// Starts an edit on the removal the action just sent.
pub fn begin(state: &mut ChatState, request_id: u64, original: String) {
    state.composer.queue_edit = Some(QueueEdit {
        request_id,
        stage: QueueEditStage::Removing,
        text: original,
    });
}

/// Moves the edit on when `request_id` is the answer it waits on. The removal and the re-queue are
/// also queue mutations, whose answers `settle_queue_mutation` has already adopted by now.
pub fn advance(state: &mut ChatState, request_id: u64, outcome: &RpcOutcome) -> Vec<Effect> {
    let Some(edit) = state.composer.queue_edit.take() else {
        return Vec::new();
    };
    if edit.request_id != request_id {
        state.composer.queue_edit = Some(edit);
        return Vec::new();
    }
    // A step that throws ends the action; the removal's failure is already on the error bar.
    let RpcOutcome::Ok { result } = outcome else {
        return Vec::new();
    };
    match edit.stage {
        QueueEditStage::Removing => {
            let text = result
                .get("prompt")
                .and_then(|prompt| prompt.get("text"))
                .and_then(Value::as_str)
                .map_or(edit.text, str::to_string);
            let request_id = state.core.allocate_request_id();
            state.composer.queue_edit = Some(QueueEdit {
                request_id,
                stage: QueueEditStage::Reading,
                text,
            });
            vec![Effect::SendRpc {
                request_id,
                method: ChatRpcMethod::ReadNativeComposer,
                params: Box::new(json!({})),
            }]
        }
        QueueEditStage::Reading => {
            let current = result.as_str().unwrap_or_default();
            let can_queue = queue_capabilities(
                state.session.queue_prompts.is_some(),
                &state.composer.transport,
            )
            .can_queue;
            if current.trim().is_empty() || !can_queue {
                // `queuePrompt` returns without a call when the capability is off.
                return insert(edit.text);
            }
            let request_id = state.core.allocate_request_id();
            state.composer.queue_mutation = Some((request_id, None));
            state.composer.queue_edit = Some(QueueEdit {
                request_id,
                stage: QueueEditStage::Queueing,
                text: edit.text,
            });
            vec![Effect::SendRpc {
                request_id,
                method: ChatRpcMethod::QueueSessionChatPrompt,
                params: Box::new(json!({ "text": current })),
            }]
        }
        QueueEditStage::Queueing => insert(edit.text),
    }
}

/// `requests.push({kind: 'composer', method: 'insert', params: {content}})`.
fn insert(content: String) -> Vec<Effect> {
    vec![Effect::SetComposerText {
        content,
        caret: None,
        from_history: false,
    }]
}
