//! The answer gate, the card's identity, and option selection.
//!
//! Port of `packages/shared/session-chat-presentation/interactive.ts`. Both chat renderers use
//! the same gate and the same labels, which is why this is rules rather than layout.

use crate::document::{QuestionControls, QuestionDraft};
use crate::questions::model::InteractivePrompt;

/// One question's saved answer while the card is open.
pub type AnswerDraft = QuestionDraft;

/// Whether a draft counts as answered: a picked option, or free text that is not just spaces.
pub fn is_answered(draft: Option<&AnswerDraft>) -> bool {
    draft.is_some_and(|draft| !draft.indices.is_empty() || !draft.other.trim().is_empty())
}

/// The card's primary button for the question at `index` of `count`.
pub fn question_answer_controls(
    drafts: &[AnswerDraft],
    index: usize,
    count: usize,
    submitting: bool,
) -> QuestionControls {
    // `count - 1` is -1 for an approval, where index 0 already counts as the last question.
    let last = index as i64 >= count as i64 - 1;
    let has_answer = drafts.iter().any(|draft| is_answered(Some(draft)));
    QuestionControls {
        has_answer,
        disabled: submitting || (last && !has_answer),
        label: if submitting {
            "Sending…"
        } else if last {
            "Send answer"
        } else if is_answered(drafts.get(index)) {
            "Next"
        } else {
            "Skip"
        }
        .to_string(),
    }
}

/// What the card remembers a dismissal by: the same question set asked again is the same card.
pub fn card_dismiss_key(prompt: Option<&InteractivePrompt>) -> Option<String> {
    Some(match prompt? {
        InteractivePrompt::Question { questions, .. } => format!(
            "question:{}:{}",
            questions.len(),
            questions
                .first()
                .map(|question| question.question.as_str())
                .unwrap_or_default()
        ),
        InteractivePrompt::Approval { tool, summary, .. } => format!(
            "approval:{}:{}",
            tool,
            summary.as_deref().unwrap_or_default()
        ),
    })
}

/// Picking an option: one of N replaces the choice, many of N toggles it and keeps them sorted.
pub fn select_question_option(
    drafts: &[AnswerDraft],
    question_index: usize,
    multi_select: bool,
    option_index: u32,
) -> Vec<AnswerDraft> {
    drafts
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            if index != question_index {
                return entry.clone();
            }
            let mut next = entry.clone();
            if !multi_select {
                next.indices = vec![option_index];
                return next;
            }
            if let Some(at) = next.indices.iter().position(|value| *value == option_index) {
                next.indices.remove(at);
            } else {
                next.indices.push(option_index);
                next.indices.sort_unstable();
            }
            next
        })
        .collect()
}
