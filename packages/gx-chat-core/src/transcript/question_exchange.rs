//! Whether a tool pair is an answered question, and what it answered.
//!
//! **This is family c's rule** (`packages/shared/session-chat-presentation/questions.ts`), ported
//! here because the transcript projection needs it in three places: the `exchange` flag that turns a
//! tool row into a card, the fold that must not hide one, and the hoisted `questions` list on a
//! completed turn. `PROGRESS.md` lists it under "to fold into family c"; when `src/questions/` lands
//! the real parser, this file is deleted and the calls move to it.

use serde_json::{json, Map, Value};

use crate::transcript::jsstr::{js_trim, js_trim_end};
use crate::transcript::tool_fold::ToolPair;

const HERMES_CLARIFY_MAX_CHOICES: usize = 4;

const RESULT_PREFIXES: [&str; 2] = ["The user answered: ", "Your questions have been answered: "];
/// Substring-matched so the exact closing-sentence wording cannot break it. No leading quote: a
/// preview answer ends in its mockup, not in a closing `"`.
const RESULT_SUFFIX_MARKERS: [&str; 2] = [". Read the answers carefully", ". You can now continue"];
/// CDXC:SessionChat 2026-09-23 WHY: Claude Code 2.1.280 follows a preview option's answer with its mockup and any note: `"Q"="Grid" selected preview:\n<mockup> notes: <text>`.
const PREVIEW_ANSWER_MARKER: &str = "\" selected preview:";
const PREVIEW_NOTE_MARKER: &str = " notes: ";
const DISMISSED_PREFIX: &str = "[User dismissed";

/// Pi's `cursor_ask_question` result envelope.
const PI_RESULT_PREFIX: &str = "User answered: ";
const PI_CANCELLED_TEXT: &str = "User cancelled the question.";

/// omp's `ask` result envelope.
const OMP_SELECTED_PREFIX: &str = "User selected: ";
const OMP_CUSTOM_PREFIX: &str = "User provided custom input:";
const OMP_NOTE_PREFIX: &str = "User added note:";
const OMP_CANCELLED_TEXT: &str = "User cancelled the selection";
const OMP_NO_SELECTION_TEXT: &str = "User did not select any options";
const OMP_MULTI_HEADER: &str = "User answers:";
const OMP_TIMEOUT_SUFFIX: &str = " (auto-selected after timeout)";

/// The tool name with everything but letters and digits removed, lowercased.
fn normalized_tool_name(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Upstream `isAskUserQuestionTool` mirror (`server/src/session_chat.rs`).
pub fn is_question_tool_name(name: &str) -> bool {
    matches!(
        normalized_tool_name(name).as_str(),
        "askuserquestion"
            | "askquestion"
            | "requestuserinput"
            | "cursoraskquestion"
            | "clarify"
            | "ask"
    )
}

/// One parsed question, the shared-contract value plus what the answer parsers need.
struct ParsedQuestion {
    /// The model-supplied id omp's `ask` echoes in its multi-question result lines.
    id: Option<String>,
    value: Value,
    text: String,
    labels: Vec<String>,
}

fn coalesce<'a>(record: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| match record.get(*key) {
        Some(Value::Null) | None => None,
        Some(value) => Some(value),
    })
}

fn parse_question_options(raw: Option<&Value>) -> Vec<Value> {
    let Some(Value::Array(items)) = raw else {
        return Vec::new();
    };
    let mut options = Vec::new();
    for option in items {
        match option {
            Value::String(label) => options.push(json!({ "label": label })),
            Value::Object(record) => {
                // Pi options carry `{label, value}`; label is what its select renders, with `value`
                // as the fallback the bridge also uses.
                let label = record
                    .get("label")
                    .and_then(Value::as_str)
                    .or_else(|| record.get("value").and_then(Value::as_str));
                let Some(label) = label else {
                    continue;
                };
                let mut entry = Map::new();
                entry.insert("label".to_string(), label.into());
                if let Some(Value::String(description)) = record.get("description") {
                    entry.insert("description".to_string(), description.clone().into());
                }
                options.push(Value::Object(entry));
            }
            _ => {}
        }
    }
    options
}

/// The canonical `{questions: [...]}` input shape (also Hermes' clarify and omp's ask), plus the
/// flat single-question shape Pi's `cursor_ask_question` sends.
fn parse_questions_with_ids(input: &Value, tool_name: &str) -> Option<Vec<ParsedQuestion>> {
    if !input.is_object() {
        return None;
    }
    let normalized = normalized_tool_name(tool_name);
    let is_hermes_clarify = normalized == "clarify";
    // Antigravity's snake_case `ask_question` normalizes to Cursor's name but is not its
    // always-checkbox panel; it says per question with `is_multi_select`.
    let is_cursor_ask_question = normalized == "askquestion" && tool_name != "ask_question";
    let candidates: Vec<Value> = match input.get("questions") {
        Some(Value::Array(items)) if !items.is_empty() => items.clone(),
        _ => vec![input.clone()],
    };
    let mut questions = Vec::new();
    for raw in &candidates {
        // Hermes tolerates bare-string batch entries (["Q1?", "Q2?"]).
        let record: Value = match raw {
            Value::Object(_) => raw.clone(),
            Value::String(text) if !js_trim(text).is_empty() => {
                json!({ "question": js_trim(text) })
            }
            _ => continue,
        };
        let text = record
            .get("question")
            .and_then(Value::as_str)
            .or_else(|| record.get("prompt").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        let mut options = parse_question_options(coalesce(&record, &["options", "choices"]));
        if is_hermes_clarify {
            options.truncate(HERMES_CLARIFY_MAX_CHOICES);
        }
        if text.is_empty() && options.is_empty() {
            continue;
        }
        // `multi_select` is Hermes' spelling, `multi` is omp's, `is_multi_select` Antigravity's;
        // Hermes honors it only when choices exist.
        let multi_select = (is_cursor_ask_question
            || record.get("multiSelect") == Some(&Value::Bool(true))
            || record.get("multi_select") == Some(&Value::Bool(true))
            || record.get("is_multi_select") == Some(&Value::Bool(true))
            || record.get("multi") == Some(&Value::Bool(true)))
            && !(is_hermes_clarify && options.is_empty());
        let mut value = Map::new();
        value.insert("question".to_string(), text.clone().into());
        if let Some(Value::String(header)) = record.get("header") {
            value.insert("header".to_string(), header.clone().into());
        }
        value.insert("multiSelect".to_string(), multi_select.into());
        if let Some(Value::Bool(allow)) = record.get("allowCustom") {
            value.insert("allowCustom".to_string(), (*allow).into());
        }
        if !tool_name.is_empty() {
            value.insert("toolName".to_string(), tool_name.into());
        }
        if let Some(recommended) = record.get("recommended").and_then(Value::as_u64) {
            value.insert("recommended".to_string(), recommended.into());
        }
        let labels: Vec<String> = options
            .iter()
            .map(|option| {
                option
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        value.insert("options".to_string(), Value::Array(options));
        let id = record
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_string);
        questions.push(ParsedQuestion {
            id,
            value: Value::Object(value),
            text,
            labels,
        });
    }
    (!questions.is_empty()).then_some(questions)
}

/// One question's answer, in the document's shape.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Answer {
    selected: Vec<usize>,
    other: Option<String>,
    dismissed: bool,
}

impl Answer {
    fn dismissed() -> Self {
        Self {
            selected: Vec::new(),
            other: None,
            dismissed: true,
        }
    }

    fn value(&self) -> Value {
        json!({
            "selectedIndices": self.selected,
            "otherText": self.other,
            "dismissed": self.dismissed,
        })
    }
}

/// Match an answer's text back onto the question's options: consume full option labels (longest
/// first) separated by ", " from the front; whatever remains is the user's own words.
///
/// Answers echo option labels verbatim, so exact prefix matching is safe; multi-select answers are
/// labels joined by ", " with any free-text answer appended the same way.
fn match_answer_to_options(question: &ParsedQuestion, text: &str) -> Answer {
    if text.starts_with(DISMISSED_PREFIX) {
        return Answer::dismissed();
    }
    let mut selected: Vec<usize> = Vec::new();
    let mut remaining = text;
    loop {
        let mut best: Option<usize> = None;
        let mut best_length = 0;
        for (index, label) in question.labels.iter().enumerate() {
            if selected.contains(&index) || label.is_empty() {
                continue;
            }
            let boundary =
                remaining.len() == label.len() || remaining.starts_with(&format!("{label}, "));
            if boundary && remaining.starts_with(label.as_str()) && label.len() > best_length {
                best = Some(index);
                best_length = label.len();
            }
        }
        let Some(index) = best else {
            break;
        };
        selected.push(index);
        remaining = &remaining[best_length..];
        if let Some(rest) = remaining.strip_prefix(", ") {
            remaining = rest;
        }
        if remaining.is_empty() {
            break;
        }
    }
    let other = js_trim(remaining);
    Answer {
        selected,
        other: (!other.is_empty()).then(|| other.to_string()),
        dismissed: false,
    }
}

/// Peel omp's trailing ` (note: …)` and ` (auto-selected after timeout)` decorations.
fn strip_omp_decorations(raw: &str) -> (Option<String>, &str) {
    let mut text = raw;
    let mut note = None;
    // `formatQuestionResult` appends the timeout suffix first and the note last.
    if let Some(body) = text.strip_suffix(')') {
        if let Some(at) = body.find(" (note: ") {
            note = Some(body[at + " (note: ".len()..].to_string());
            text = &text[..at];
        }
    }
    if let Some(shorter) = text.strip_suffix(OMP_TIMEOUT_SUFFIX) {
        text = shorter;
    }
    (note, text)
}

fn parse_omp_single_answer(question: &ParsedQuestion, trimmed: &str) -> Option<Answer> {
    if trimmed == OMP_CANCELLED_TEXT {
        return Some(Answer::dismissed());
    }
    if trimmed == OMP_NO_SELECTION_TEXT {
        return Some(Answer::default());
    }
    let lines: Vec<&str> = trimmed.split('\n').collect();
    let opens_as_omp = lines.first().is_some_and(|line| {
        line.starts_with(OMP_SELECTED_PREFIX)
            || line.starts_with(OMP_CUSTOM_PREFIX)
            || line.starts_with(OMP_NOTE_PREFIX)
    });
    if !opens_as_omp {
        return None;
    }
    let mut selected: Vec<usize> = Vec::new();
    let mut extras: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(labels) = line.strip_prefix(OMP_SELECTED_PREFIX) {
            let labels = labels.strip_suffix(OMP_TIMEOUT_SUFFIX).unwrap_or(labels);
            let matched = match_answer_to_options(question, labels);
            selected = matched.selected;
            if let Some(other) = matched.other {
                extras.push(other);
            }
            index += 1;
            continue;
        }
        let prefix = if line.starts_with(OMP_CUSTOM_PREFIX) {
            Some(OMP_CUSTOM_PREFIX)
        } else if line.starts_with(OMP_NOTE_PREFIX) {
            Some(OMP_NOTE_PREFIX)
        } else {
            None
        };
        let Some(prefix) = prefix else {
            index += 1;
            continue;
        };
        let rest = &line[prefix.len()..];
        let mut body = rest.strip_prefix(' ').unwrap_or(rest).to_string();
        index += 1;
        if body.is_empty() {
            // Multi-line body: the following lines are two-space indented.
            let mut body_lines: Vec<&str> = Vec::new();
            while index < lines.len() && lines[index].starts_with("  ") {
                body_lines.push(&lines[index][2..]);
                index += 1;
            }
            body = body_lines.join("\n");
        }
        if !body.is_empty() {
            extras.push(body);
        }
    }
    Some(Answer {
        selected,
        other: (!extras.is_empty()).then(|| extras.join("\n")),
        dismissed: false,
    })
}

fn parse_omp_answer_value(question: &ParsedQuestion, raw: &str) -> Answer {
    let (note, text) = strip_omp_decorations(raw);
    if text == "(cancelled)" {
        return Answer {
            selected: Vec::new(),
            other: note,
            dismissed: true,
        };
    }
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        let custom = &text[1..text.len() - 1];
        let joined = [Some(custom.to_string()), note]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        return Answer {
            selected: Vec::new(),
            other: (!joined.is_empty()).then_some(joined),
            dismissed: false,
        };
    }
    let list_body =
        (text.starts_with('[') && text.ends_with(']')).then(|| &text[1..text.len() - 1]);
    let matched = match_answer_to_options(question, list_body.unwrap_or(text));
    let other: Vec<String> = [matched.other, note].into_iter().flatten().collect();
    Answer {
        selected: matched.selected,
        other: (!other.is_empty()).then(|| other.join("\n")),
        dismissed: false,
    }
}

fn parse_omp_multi_answers(
    entries: &[ParsedQuestion],
    trimmed: &str,
) -> Option<Vec<Option<Answer>>> {
    let body = trimmed.strip_prefix(OMP_MULTI_HEADER)?;
    // Locate each question's `\n<id>: ` marker in order; custom inputs are quoted but not escaped,
    // so marker slicing is the only reliable read.
    let mut found: Vec<(usize, usize, usize)> = Vec::new();
    let mut from = 0;
    for (index, entry) in entries.iter().enumerate() {
        let Some(id) = &entry.id else {
            continue;
        };
        let marker = format!("\n{id}: ");
        if let Some(at) = body[from.min(body.len())..].find(&marker) {
            let start = from + at;
            found.push((index, start, start + marker.len()));
            from = start + marker.len();
        }
    }
    if found.is_empty() {
        return None;
    }
    let mut answers: Vec<Option<Answer>> = entries.iter().map(|_| None).collect();
    for (position, (index, _, end)) in found.iter().enumerate() {
        let raw = match found.get(position + 1) {
            Some((_, next_start, _)) => &body[*end..*next_start],
            None => &body[*end..],
        };
        answers[*index] = Some(parse_omp_answer_value(&entries[*index], js_trim_end(raw)));
    }
    Some(answers)
}

fn hermes_response_to_answer(
    question: &ParsedQuestion,
    response: Option<&Value>,
    timed_out: bool,
) -> Answer {
    if let Some(Value::Array(items)) = response {
        let mut selected: Vec<usize> = Vec::new();
        let mut extras: Vec<String> = Vec::new();
        for item in items {
            let Some(text) = item.as_str().filter(|text| !text.is_empty()) else {
                continue;
            };
            match question
                .labels
                .iter()
                .enumerate()
                .find(|(index, label)| label.as_str() == text && !selected.contains(index))
            {
                Some((index, _)) => selected.push(index),
                None => extras.push(text.to_string()),
            }
        }
        return Answer {
            selected,
            other: (!extras.is_empty()).then(|| extras.join(", ")),
            dismissed: false,
        };
    }
    let text = response
        .and_then(Value::as_str)
        .map(js_trim)
        .unwrap_or_default();
    if text.is_empty() {
        return Answer {
            selected: Vec::new(),
            other: None,
            dismissed: timed_out,
        };
    }
    match_answer_to_options(question, text)
}

fn parse_hermes_answers(entries: &[ParsedQuestion], trimmed: &str) -> Option<Vec<Option<Answer>>> {
    if !trimmed.starts_with('{') {
        return None;
    }
    let parsed: Value = serde_json::from_str(trimmed).ok()?;
    let record = parsed.as_object()?;
    let timed_out = record.get("timed_out") == Some(&Value::Bool(true));
    if let Some(Value::Array(responses)) = record.get("responses") {
        let mut answers: Vec<Option<Answer>> = entries.iter().map(|_| None).collect();
        for (index, row) in responses.iter().enumerate() {
            let (Some(question), Some(row)) = (entries.get(index), row.as_object()) else {
                continue;
            };
            answers[index] = Some(hermes_response_to_answer(
                question,
                row.get("user_response"),
                timed_out,
            ));
        }
        return answers.iter().any(Option::is_some).then_some(answers);
    }
    if record.contains_key("user_response") && entries.len() == 1 {
        return Some(vec![Some(hermes_response_to_answer(
            &entries[0],
            record.get("user_response"),
            false,
        ))]);
    }
    None
}

/// Antigravity's `A<n>: <answer>` lines, `User Skipped` for a skipped question.
fn parse_antigravity_answers(
    entries: &[ParsedQuestion],
    trimmed: &str,
) -> Option<Vec<Option<Answer>>> {
    let markers =
        crate::questions::exchange_answers::antigravity_answer_markers(trimmed, entries.len())?;
    Some(
        markers
            .iter()
            .enumerate()
            .map(|(index, (_, end))| {
                let stop = markers.get(index + 1).map_or(trimmed.len(), |next| next.0);
                let value = js_trim(&trimmed[*end..stop]);
                (value != "User Skipped").then(|| match_answer_to_options(&entries[index], value))
            })
            .collect(),
    )
}

/// `"…"` body between the known prefix and the closing sentence.
fn strip_answer_envelope(output: &str) -> Option<&str> {
    let prefix = RESULT_PREFIXES
        .into_iter()
        .find(|candidate| output.starts_with(candidate))?;
    let mut body = &output[prefix.len()..];
    for marker in RESULT_SUFFIX_MARKERS {
        if let Some(at) = body.rfind(marker) {
            body = &body[..at];
            break;
        }
    }
    Some(body)
}

fn parse_answers(entries: &[ParsedQuestion], output: &str) -> Option<Vec<Option<Answer>>> {
    let trimmed = js_trim(output);
    if entries.len() == 1 {
        let first = &entries[0];
        if trimmed == PI_CANCELLED_TEXT {
            return Some(vec![Some(Answer::dismissed())]);
        }
        if let Some(rest) = trimmed.strip_prefix(PI_RESULT_PREFIX) {
            if !trimmed.contains('\n') {
                return Some(vec![Some(match_answer_to_options(first, rest))]);
            }
        }
        if let Some(single) = parse_omp_single_answer(first, trimmed) {
            return Some(vec![Some(single)]);
        }
    }
    if let Some(multi) = parse_omp_multi_answers(entries, trimmed) {
        return Some(multi);
    }
    if let Some(hermes) = parse_hermes_answers(entries, trimmed) {
        return Some(hermes);
    }
    if let Some(antigravity) = parse_antigravity_answers(entries, trimmed) {
        return Some(antigravity);
    }
    let body = strip_answer_envelope(trimmed)?;
    // Locate each question's `"question"="` marker in order; a question whose marker is missing was
    // skipped and keeps a null entry.
    let mut found: Vec<(usize, usize, usize)> = Vec::new();
    let mut from = 0;
    for (index, entry) in entries.iter().enumerate() {
        if entry.text.is_empty() {
            continue;
        }
        let marker = format!("\"{}\"=\"", entry.text);
        if let Some(at) = body[from.min(body.len())..].find(&marker) {
            let start = from + at;
            found.push((index, start, start + marker.len()));
            from = start + marker.len();
        }
    }
    if found.is_empty() {
        return None;
    }
    let mut answers: Vec<Option<Answer>> = entries.iter().map(|_| None).collect();
    for (position, (index, _, end)) in found.iter().enumerate() {
        let raw = match found.get(position + 1) {
            Some((_, next_start, _)) => &body[*end..*next_start],
            None => &body[*end..],
        };
        let mut raw = js_trim_end(raw);
        raw = raw.strip_suffix(',').unwrap_or(raw);
        raw = raw.strip_suffix('"').unwrap_or(raw);
        answers[*index] = Some(match_claude_answer(&entries[*index], raw));
    }
    Some(answers)
}

/// An answered question tool pair as the card renders it; anything else (pending question, error
/// result, unparseable input) is `None` and keeps the generic tool row.
pub fn answered_question_exchange(pair: &ToolPair<'_>) -> Option<Value> {
    if pair.call.is_none() || pair.result.is_none() || pair.result_is_error() {
        return None;
    }
    let name = pair.call_name()?;
    if !is_question_tool_name(name) {
        return None;
    }
    let entries = parse_questions_with_ids(pair.call_input()?, name)?;
    let output = js_trim(pair.result_output().unwrap_or_default()).to_string();
    if output.is_empty() {
        return None;
    }
    let answers = parse_answers(&entries, &output);
    Some(json!({
        "questions": entries.iter().map(|entry| entry.value.clone()).collect::<Vec<_>>(),
        "answers": match &answers {
            Some(answers) => Value::Array(
                answers.iter().map(|answer| answer.as_ref().map_or(Value::Null, Answer::value)).collect(),
            ),
            None => Value::Null,
        },
        "fallbackText": if answers.is_none() { Value::String(output) } else { Value::Null },
    }))
}

/// A Claude answer, with a preview option's mockup dropped and its note kept as the user's words.
fn match_claude_answer(question: &ParsedQuestion, raw: &str) -> Answer {
    let Some(preview_at) = raw.find(PREVIEW_ANSWER_MARKER) else {
        return match_answer_to_options(question, raw);
    };
    let preview = &raw[preview_at + PREVIEW_ANSWER_MARKER.len()..];
    let note = preview
        .rfind(PREVIEW_NOTE_MARKER)
        .map(|at| js_trim(&preview[at + PREVIEW_NOTE_MARKER.len()..]))
        .unwrap_or_default();
    let mut matched = match_answer_to_options(question, &raw[..preview_at]);
    let other: Vec<&str> = matched
        .other
        .as_deref()
        .into_iter()
        .chain((!note.is_empty()).then_some(note))
        .collect();
    matched.other = (!other.is_empty()).then(|| other.join("\n"));
    matched
}
