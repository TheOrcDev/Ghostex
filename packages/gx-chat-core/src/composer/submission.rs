//! Sending: what a send, a queue and a compact do, and what happens to the text when one fails.
//!
//! Port of `packages/shared/session-chat-controller/submission.ts` and `draft-handoff.ts`.

use serde::{Deserialize, Serialize};

use crate::composer::queue::DraftVersion;

/// What the composer is doing with its text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubmissionMode {
    #[default]
    Send,
    Queue,
    Compact,
}

/// The steps a submission runs through, in order.
///
/// CDXC:Drafts 2026-09-17 DECISION:
/// User: preserve unsent messages and retire sent drafts by identity and revision, including older
/// copies containing deleted text. Await the submitted revision before delivery; typing after
/// submission belongs to a new identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubmissionPhase {
    /// Push the draft so the submitted revision is durable before anything is delivered.
    SaveDraft,
    /// Deliver the message itself.
    DeliverMessage,
    /// Queue the text after `/compact` was accepted.
    QueueAfterCompact,
}

/// One step of a submission, as the core hands it to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmissionStep {
    /// Save the draft first.
    PushDraft {
        text: String,
        version: Option<DraftVersion>,
    },
    /// Send text to the agent. `/compact` is sent this way too.
    Send {
        text: String,
        version: Option<DraftVersion>,
    },
    /// Queue text through gxserver.
    Queue {
        text: String,
        version: Option<DraftVersion>,
    },
}

/// The steps a submission takes, in order.
///
/// A `compact` sends `/compact` first and then queues the text, which is why the queue endpoint is
/// required for it as well as for a plain queue.
pub fn submission_steps(
    text: &str,
    version: Option<&DraftVersion>,
    mode: SubmissionMode,
    can_push: bool,
    can_queue: bool,
) -> Result<Vec<SubmissionStep>, &'static str> {
    let version = version.cloned();
    let mut steps = Vec::new();
    if can_push {
        steps.push(SubmissionStep::PushDraft {
            text: text.to_string(),
            version: version.clone(),
        });
    }
    if mode == SubmissionMode::Compact {
        steps.push(SubmissionStep::Send {
            text: "/compact".to_string(),
            version: None,
        });
    }
    if mode == SubmissionMode::Send {
        steps.push(SubmissionStep::Send {
            text: text.to_string(),
            version,
        });
    } else {
        if !can_queue {
            return Err("This session cannot queue prompts.");
        }
        steps.push(SubmissionStep::Queue {
            text: if mode == SubmissionMode::Queue {
                text.trim().to_string()
            } else {
                text.to_string()
            },
            version,
        });
    }
    Ok(steps)
}

/// The text to put back when a send did not leave.
pub fn restore_undelivered_text(submitted: &str, current: &str) -> String {
    // A lone Side Chat pill is the box's only content; the refused message already carries it.
    if current.is_empty()
        || current == submitted
        || current == crate::composer::side_chat::SIDE_CHAT_PREFIX
    {
        submitted.to_string()
    } else {
        format!("{submitted}\n{current}")
    }
}

/// What a delayed draft transfer should do with the text already in the composer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandoffDisposition {
    /// The composer already holds this draft.
    Current,
    /// Different text is typed here; offer the incoming draft rather than replacing it.
    Conflict,
    /// Nothing to lose; take it.
    Accept,
}

/// A stored draft, as far as the handoff rule reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredDraft {
    pub text: String,
    pub version: Option<DraftVersion>,
    pub parked: bool,
}

/// A delayed transfer must never replace text typed after the view switch.
pub fn classify_draft_handoff(
    content: &str,
    version: Option<&DraftVersion>,
    current: &str,
    stored: Option<&StoredDraft>,
    parked: bool,
) -> HandoffDisposition {
    if let (Some(version), Some(stored)) = (version, stored) {
        if let Some(stored_version) = &stored.version {
            if stored_version.draft_id == version.draft_id
                && !stored.parked
                && !parked
                && current == stored.text
                && stored_version.revision >= version.revision
            {
                return HandoffDisposition::Current;
            }
        }
    }
    if !current.is_empty() && current != content {
        HandoffDisposition::Conflict
    } else {
        HandoffDisposition::Accept
    }
}
