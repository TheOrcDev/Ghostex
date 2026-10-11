//! CDXC:SessionChat 2026-10-04 WHY:
//! Freebuff has no hooks and often saves a pending `ask_user` call to `chat-messages.json` only after it is answered, so the open form on screen is the reliable source of the question card; when the saved chat does hold the pending call, the card shows the whole call instead.
//! The form expands one question at a time (`▼ question`, `○`/`●` or `☐`/`☑` rows, a final `Custom` row, then a `Submit` button beside `↑↓ navigate • Enter select`), and answering one opens the next.
//! SEE-ALSO: server/src/session_chat_freebuff.rs; Freebuff's form keys are in CodebuffAI/codebuff `cli/src/components/ask-user/index.tsx`.

use crate::session_chat::{
    SessionChatInteractivePrompt, SessionChatQuestion, SessionChatQuestionOption,
    SessionChatQuestionSelection,
};
use crate::session_chat_send::AskAnswerKeyGroup;

const FREEBUFF_ASK_TOOL: &str = "ask_user";
const ENTER: &str = "\r";
const DOWN: &str = "\u{1b}[B";
const LEFT: &str = "\u{1b}[D";
const RIGHT: &str = "\u{1b}[C";
/// The form's hint line sits just above the input box's bottom border and its footer.
const HINT_MAX_LINES_FROM_BOTTOM: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreebuffQuestionForm {
    pub question: SessionChatQuestion,
    /// Another question is still collapsed below this one, so answering this one does not submit the form.
    pub has_more: bool,
}

fn form_lines(screen_text: &str) -> Vec<String> {
    screen_text
        .lines()
        .map(|line| {
            let line = crate::session_chat_options::strip_ansi_sgr(line);
            let mut line = line.trim();
            line = line.strip_prefix('│').unwrap_or(line);
            line = line.strip_suffix('│').unwrap_or(line);
            line.trim().to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn option_row(line: &str) -> Option<(bool, &str)> {
    let mut chars = line.chars();
    let marker = chars.next()?;
    let multi = match marker {
        '○' | '●' => false,
        '☐' | '☑' => true,
        _ => return None,
    };
    Some((multi, chars.as_str().trim()))
}

pub fn detect_freebuff_question_form(screen_text: &str) -> Option<FreebuffQuestionForm> {
    let lines = form_lines(screen_text);
    let hint = lines
        .iter()
        .rposition(|line| line.contains("↑↓ navigate") && line.contains("Enter select"))?;
    if lines.len() - hint > HINT_MAX_LINES_FROM_BOTTOM {
        return None;
    }
    let header = lines[..hint]
        .iter()
        .rposition(|line| line.starts_with("▼ "))?;
    let mut text = lines[header]["▼ ".len()..].trim();
    if let Some((number, rest)) = text.split_once(". ") {
        if number.parse::<usize>().is_ok() {
            text = rest.trim();
        }
    }
    let mut options = Vec::new();
    let mut multi_select = false;
    let mut allow_custom = false;
    let mut has_more = false;
    for line in &lines[header + 1..hint] {
        if line.starts_with("▶ ") {
            has_more = true;
            break;
        }
        let Some((multi, label)) = option_row(line) else {
            // Descriptions under the focused row and Custom's text box.
            continue;
        };
        multi_select = multi;
        options.push(label.to_string());
    }
    if options.last().map(String::as_str) == Some("Custom") {
        options.pop();
        allow_custom = true;
    }
    if text.is_empty() || (options.is_empty() && !allow_custom) {
        return None;
    }
    Some(FreebuffQuestionForm {
        question: SessionChatQuestion {
            question: text.to_string(),
            header: None,
            multi_select,
            allow_custom: Some(allow_custom),
            tool_name: Some(FREEBUFF_ASK_TOOL.to_string()),
            recommended: None,
            preview_layout: false,
            options: options
                .into_iter()
                .map(|label| SessionChatQuestionOption {
                    label,
                    description: None,
                    preview: None,
                })
                .collect(),
        },
        has_more,
    })
}

pub fn detect_freebuff_question_prompt(
    agent: Option<&str>,
    screen_text: &str,
) -> Option<SessionChatInteractivePrompt> {
    if agent.map(str::trim) != Some("freebuff") {
        return None;
    }
    let form = detect_freebuff_question_form(screen_text)?;
    Some(SessionChatInteractivePrompt::Question {
        questions: vec![form.question],
        tool_use_id: None,
    })
}

/// One step per arrow: Freebuff reads every key of one write against the same highlight, so two
/// Downs written together move it only one row.
fn downs(count: usize) -> Vec<AskAnswerKeyGroup> {
    vec![AskAnswerKeyGroup::Raw(DOWN.to_string()); count]
}

/// Keys that answer one question once its rows are open with the highlight on the first row,
/// or `None` when the card left it unanswered. Picking a row of a single-choice question opens
/// the next question, or moves to Submit after the last one; a multiple-choice question stays
/// open until the highlight moves past its Custom row.
fn question_keys(
    question: &SessionChatQuestion,
    selection: Option<&SessionChatQuestionSelection>,
) -> Option<Vec<AskAnswerKeyGroup>> {
    let custom_row = question.options.len();
    let other = selection
        .and_then(|selection| selection.other.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty() && question.allow_custom != Some(false));
    let mut picked: Vec<usize> = selection
        .map(|selection| selection.indices.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|index| *index < custom_row)
        .collect();
    picked.sort_unstable();
    picked.dedup();
    if picked.is_empty() && other.is_none() {
        return None;
    }
    let mut groups = Vec::new();
    if question.multi_select {
        let mut row = 0;
        for index in picked {
            groups.extend(downs(index - row));
            groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
            row = index;
        }
        if let Some(other) = other {
            groups.extend(downs(custom_row - row));
            groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
            groups.push(AskAnswerKeyGroup::Text(other.to_string()));
            groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
            row = custom_row;
        }
        groups.extend(downs(custom_row - row + 1));
    } else if let Some(other) = other {
        groups.extend(downs(custom_row));
        groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
        groups.push(AskAnswerKeyGroup::Text(other.to_string()));
        groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
    } else {
        groups.extend(downs(picked[0]));
        groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
    }
    Some(groups)
}

/// Keys for the card's `questions` from the one `form` has open onward. The card holds that one
/// question when it was read off the screen, or the whole `ask_user` call when Freebuff's saved
/// chat already has it. Left then Right re-opens the open question with the highlight on its first
/// row, whatever was moved in the terminal; each answer opens the next question at its first row.
/// An unanswered question stops the keys there and leaves the rest to the terminal.
pub fn build_freebuff_ask_answer_keys(
    form: &FreebuffQuestionForm,
    questions: &[SessionChatQuestion],
    selections: &[SessionChatQuestionSelection],
) -> Vec<AskAnswerKeyGroup> {
    let Some(start) = questions
        .iter()
        .position(|question| question.question == form.question.question)
    else {
        return Vec::new();
    };
    let mut groups = vec![
        AskAnswerKeyGroup::Raw(LEFT.to_string()),
        AskAnswerKeyGroup::Raw(RIGHT.to_string()),
    ];
    for (index, question) in questions.iter().enumerate().skip(start) {
        // The screen's own rows for the open question; the card's for the ones after it.
        let question = if index == start {
            &form.question
        } else {
            question
        };
        let Some(keys) = question_keys(question, selections.get(index)) else {
            return if index == start { Vec::new() } else { groups };
        };
        groups.extend(keys);
    }
    if questions.len() > 1 || !form.has_more {
        // The highlight is on Submit now.
        groups.push(AskAnswerKeyGroup::Raw(ENTER.to_string()));
    }
    groups
}
