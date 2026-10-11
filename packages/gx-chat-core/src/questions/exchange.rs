//! Reading an asking tool's own input back into questions.
//!
//! Port of the parsing half of `packages/shared/session-chat-presentation/questions.ts`: the
//! canonical `{questions: [...]}` shape (also Hermes' clarify and omp's ask), plus the flat
//! single-question shape Pi's `cursor_ask_question` sends, with `prompt` and `choices` accepted as
//! the aliases the bridge normalizes.

use serde_json::Value;

use crate::questions::model::{Question, QuestionOption};
use crate::transcript::jsstr::js_trim;

/// Hermes' clarify tool hard-caps `choices` at 4 before the terminal panel renders, so the
/// answered card mirrors what was actually offered.
const HERMES_CLARIFY_MAX_CHOICES: usize = 4;

/// One parsed question plus the model-supplied id omp's `ask` echoes in its multi-question result
/// lines (`id: value`). Only the exchange parser needs the id, so it rides beside the
/// shared-contract question rather than on it.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedQuestion {
    pub id: Option<String>,
    pub question: Question,
}

/// Upstream `isAskUserQuestionTool` mirror (`session_chat.rs`).
pub fn is_question_tool_name(name: &str) -> bool {
    let normalized = normalize_tool_name(name);
    matches!(
        normalized.as_str(),
        "askuserquestion"
            | "askquestion"
            | "requestuserinput"
            | "cursoraskquestion"
            // Hermes Agent / oh-my-pi.
            | "clarify"
            | "ask"
    )
}

/// `name.replace(/[^a-zA-Z0-9]/g, '').toLowerCase()`.
fn normalize_tool_name(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_lowercase()
}

/// The questions an asking tool's input carries, with the ids its result lines echo.
pub fn parse_questions_with_ids(
    input: &Value,
    tool_name: Option<&str>,
) -> Option<Vec<ParsedQuestion>> {
    // `typeof input === 'object'`: an array passes that test in JavaScript too, and falls through
    // to producing nothing rather than being refused here.
    if !matches!(input, Value::Object(_) | Value::Array(_)) {
        return None;
    }
    let normalized = tool_name.map(normalize_tool_name).unwrap_or_default();
    let is_hermes_clarify = normalized == "clarify";
    // Antigravity's snake_case `ask_question` normalizes to Cursor's name but is not its
    // always-checkbox panel; it says per question with `is_multi_select`.
    let is_cursor_ask_question = normalized == "askquestion" && tool_name != Some("ask_question");
    let raw_questions = input.get("questions");
    let candidates: Vec<&Value> = match raw_questions {
        Some(Value::Array(items)) if !items.is_empty() => items.iter().collect(),
        _ => vec![input],
    };
    let mut questions = Vec::new();
    for raw in candidates {
        // Hermes tolerates bare-string batch entries (["Q1?", "Q2?"]).
        let bare = match raw {
            Value::String(text) if !js_trim(text).is_empty() => Some(js_trim(text).to_string()),
            _ => None,
        };
        if !matches!(raw, Value::Object(_) | Value::Array(_)) && bare.is_none() {
            continue;
        }
        let text = match &bare {
            Some(text) => text.clone(),
            None => raw
                .get("question")
                .and_then(Value::as_str)
                .or_else(|| raw.get("prompt").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string(),
        };
        let mut options = match &bare {
            Some(_) => Vec::new(),
            None => parse_question_options(
                nullish(raw.get("options")).or_else(|| nullish(raw.get("choices"))),
            ),
        };
        if is_hermes_clarify {
            options.truncate(HERMES_CLARIFY_MAX_CHOICES);
        }
        if text.is_empty() && options.is_empty() {
            continue;
        }
        // `multi_select` is Hermes' spelling, `multi` is omp's, `is_multi_select` Antigravity's;
        // Hermes honors it only when choices exist.
        let flagged = is_cursor_ask_question
            || is_true(raw.get("multiSelect"))
            || is_true(raw.get("multi_select"))
            || is_true(raw.get("is_multi_select"))
            || is_true(raw.get("multi"));
        let multi_select = flagged && !(is_hermes_clarify && options.is_empty());
        let id = raw
            .get("id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        questions.push(ParsedQuestion {
            id: if bare.is_some() { None } else { id },
            question: Question {
                question: text,
                header: raw
                    .get("header")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .filter(|_| bare.is_none()),
                multi_select,
                allow_custom: if bare.is_some() {
                    None
                } else {
                    raw.get("allowCustom").and_then(Value::as_bool)
                },
                tool_name: tool_name
                    .filter(|name| !name.is_empty())
                    .map(str::to_string),
                recommended: if bare.is_some() {
                    None
                } else {
                    raw.get("recommended").and_then(recommended_index)
                },
                options,
            },
        });
    }
    if questions.is_empty() {
        None
    } else {
        Some(questions)
    }
}

/// The canonical entry point: the questions alone, without the result-line ids.
pub fn parse_questions_input(input: &Value, tool_name: Option<&str>) -> Option<Vec<Question>> {
    parse_questions_with_ids(input, tool_name)
        .map(|entries| entries.into_iter().map(|entry| entry.question).collect())
}

/// JavaScript's `??`: `null` and a missing key both fall through to the next candidate.
fn nullish(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

fn is_true(value: Option<&Value>) -> bool {
    value == Some(&Value::Bool(true))
}

/// `Number.isInteger(value) && value >= 0`.
fn recommended_index(value: &Value) -> Option<u64> {
    let number = value.as_f64()?;
    if number.fract() != 0.0 || number < 0.0 || number > u64::MAX as f64 {
        return None;
    }
    Some(number as u64)
}

/// Option rows, accepting bare strings and the `{label, value, description}` records Pi sends.
fn parse_question_options(raw: Option<&Value>) -> Vec<QuestionOption> {
    let Some(Value::Array(items)) = raw else {
        return Vec::new();
    };
    let mut options = Vec::new();
    for option in items {
        if let Value::String(label) = option {
            options.push(QuestionOption {
                label: label.clone(),
                description: None,
            });
            continue;
        }
        if !matches!(option, Value::Object(_) | Value::Array(_)) {
            continue;
        }
        // Pi options carry `{label, value}`; label is what its select renders, with `value` as
        // the fallback the bridge also uses.
        let Some(label) = option
            .get("label")
            .and_then(Value::as_str)
            .or_else(|| option.get("value").and_then(Value::as_str))
        else {
            continue;
        };
        options.push(QuestionOption {
            label: label.to_string(),
            description: option
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    options
}
