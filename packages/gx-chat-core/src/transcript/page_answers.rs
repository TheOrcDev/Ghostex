//! Answers a reader sent from an agent's HTML page (`ghostex.submit(...)` in a page published with
//! `ghostex show`), as gxserver types them into the publishing session:
//!
//! ```text
//! <ghostex-page-answers page="<32 hex id>" title="Open questions">
//! { …the answers as JSON… }
//! </ghostex-page-answers>
//! ```
//!
//! The chat draws that user message as an answers card instead of a bubble of JSON, and marks the
//! page's own card Answered (`answered_page_ids`, the document's `answeredPages`).
//!
//! CDXC:SessionChat 2026-10-11 SEE-ALSO:
//! The envelope is written by server/src/visual_pages/answers.rs; the cards are drawn by apps/desktop/src/app/native_chat/page_answers.rs and visual.rs, and on the phone by apps/mobile/app/src/chat/native/transcript/PageAnswersCard.tsx and VisualBlock.tsx. Change them together.

use ghostex_gx_protocol::{ChatMessage, ChatRole};
use serde_json::{json, Value};

use crate::transcript::jsstr::js_trim;
use crate::transcript::prose::prose_markdown;

const OPEN: &str = "<ghostex-page-answers";
const CLOSE: &str = "</ghostex-page-answers>";
/// Rows the card lists before it summarises the rest.
const MAX_ROWS: usize = 40;
const MAX_VALUE_CHARS: usize = 600;
/// How the question kit (server/src/visual_pages/kit.rs) names a question's note.
const NOTE_SUFFIX: &str = " (note)";

/// One answered page, as its message carries it.
#[derive(Clone, Debug, PartialEq)]
pub struct PageAnswers {
    pub page_id: String,
    pub title: String,
    pub answers: Value,
}

fn attribute(head: &str, name: &str) -> Option<String> {
    let start = head.find(&format!("{name}=\""))? + name.len() + 2;
    let end = head[start..].find('"')? + start;
    Some(
        head[start..end]
            .replace("&quot;", "\"")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&"),
    )
}

fn is_page_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The answers a message holds, when the whole message is one answers envelope.
pub fn parse_page_answers(text: &str) -> Option<PageAnswers> {
    let text = js_trim(text);
    let rest = text.strip_prefix(OPEN)?;
    let head_end = rest.find('>')?;
    let head = &rest[..head_end];
    let body = rest[head_end + 1..].strip_suffix(CLOSE)?;
    let page_id = attribute(head, "page").filter(|id| is_page_id(id))?;
    let title = attribute(head, "title")
        .map(|title| js_trim(&title).to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Page".to_string());
    let body = js_trim(body);
    let answers = serde_json::from_str(body).unwrap_or_else(|_| Value::String(body.to_string()));
    Some(PageAnswers {
        page_id,
        title,
        answers,
    })
}

/// The pages whose answers this transcript received, in the order they arrived.
pub fn answered_page_ids(messages: &[ChatMessage]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for message in messages
        .iter()
        .filter(|message| message.role == ChatRole::User)
    {
        if let Some(answers) = parse_page_answers(&prose_markdown(&message.blocks)) {
            if !ids.contains(&answers.page_id) {
                ids.push(answers.page_id);
            }
        }
    }
    ids
}

fn clip(text: String) -> String {
    if text.chars().count() <= MAX_VALUE_CHARS {
        return text;
    }
    let mut clipped: String = text.chars().take(MAX_VALUE_CHARS).collect();
    clipped.push('…');
    clipped
}

/// A value as the card shows it: text as written, a list joined, anything else as compact JSON.
fn display_value(value: &Value) -> String {
    clip(match value {
        Value::String(text) => text.clone(),
        Value::Null => "Not answered".to_string(),
        Value::Bool(true) => "Yes".to_string(),
        Value::Bool(false) => "No".to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(items)
            if items
                .iter()
                .all(|item| !item.is_object() && !item.is_array()) =>
        {
            items
                .iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        }
        other => other.to_string(),
    })
}

/// A field name as a label: `preferred_name` and `preferredName` read as "Preferred name". A key
/// that is already words (the question kit keys answers by question title) is shown as written.
fn display_label(key: &str) -> String {
    if !key
        .chars()
        .all(|character| character.is_alphanumeric() || character == '_' || character == '-')
    {
        return js_trim(key).to_string();
    }
    let mut words = String::new();
    let mut previous_lower = false;
    for character in key.chars() {
        if character == '_' || character == '-' {
            words.push(' ');
            previous_lower = false;
            continue;
        }
        if character.is_uppercase() && previous_lower {
            words.push(' ');
        }
        previous_lower = character.is_lowercase();
        words.extend(character.to_lowercase());
    }
    let words = js_trim(&words).to_string();
    let mut characters = words.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => key.to_string(),
    }
}

/// The message's `pageAnswers` key: `{pageId, title, rows: [{label, value, note?}], more}`. The
/// question kit sends a question's note as `"<key> (note)"`; it is shown under that question's
/// answer instead of as a row of its own.
pub fn project_page_answers(answers: &PageAnswers) -> Value {
    let rows: Vec<Value> = match &answers.answers {
        Value::Object(fields) => fields
            .iter()
            .filter(|(key, _)| {
                key.strip_suffix(NOTE_SUFFIX)
                    .is_none_or(|question| !fields.contains_key(question))
            })
            .map(|(key, value)| {
                let mut row = json!({"label": display_label(key), "value": display_value(value)});
                if let Some(note) = fields.get(&format!("{key}{NOTE_SUFFIX}")) {
                    row["note"] = json!(display_value(note));
                }
                row
            })
            .collect(),
        other => vec![json!({"label": "", "value": display_value(other)})],
    };
    let more = rows.len().saturating_sub(MAX_ROWS);
    json!({
        "pageId": answers.page_id,
        "title": answers.title,
        "rows": rows.into_iter().take(MAX_ROWS).collect::<Vec<_>>(),
        "more": more,
    })
}
