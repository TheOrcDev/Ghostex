//! The Save to Markdown sheet, ported from
//! `packages/shared/session-chat-controller/save-markdown.ts` plus the two Docs calls
//! `packages/shared/project-docs.ts` makes for it.
//!
//! One shared dialog lifecycle stops a stale listing replacing a typed name or a later sheet: every
//! open and close bumps a generation, and an answer whose generation is no longer current is
//! dropped.
//!
//! The Markdown itself is never built here. The renderer hands the text over with
//! `markdownSaveOpen` and it is written back byte for byte, so a saved file is exactly the one the
//! transcript row copied.

use serde_json::{json, Map, Value};

use crate::effect::Effect;
use crate::extras::save_markdown_paths::{
    folder_path_error, local_date_directory, markdown_stem_error, normalized_folder_path,
    normalized_markdown_stem, suggested_markdown_stem,
};
use crate::state::{
    ChatContext, SaveMarkdownRequest, SaveMarkdownSheet, SaveMarkdownStage, SaveMarkdownState,
};
use crate::wire::ChatRpcMethod;

/// `markdownSaveOpen`: start a sheet and ask Docs what is already in the project.
pub fn open(
    state: &mut SaveMarkdownState,
    title: &str,
    markdown: &str,
    context: &ChatContext,
    request_id: u64,
) -> Vec<Effect> {
    if state.sheet.as_ref().is_some_and(|sheet| sheet.saving) {
        return Vec::new();
    }
    state.generation += 1;
    state.title = title.to_string();
    state.markdown = markdown.to_string();
    state.paths = None;
    state.save_request = None;
    let folder = local_date_directory(context.now_ms, context.utc_offset_minutes);
    state.sheet = Some(SaveMarkdownSheet {
        file_name: suggested_markdown_stem(title, &folder, &[]),
        folder,
        suggested: true,
        loading: true,
        saving: false,
        folder_error: None,
        file_name_error: None,
        listing_error: None,
    });
    let correlation = correlation_id(state, "list-message-markdown");
    state.list_request = Some((request_id, state.generation));
    vec![docs_call(
        request_id,
        json!({ "action": "list", "requestId": correlation }),
    )]
}

/// `markdownSaveCancel`: close the sheet unless a save is in flight.
pub fn close(state: &mut SaveMarkdownState) {
    if state.sheet.as_ref().is_some_and(|sheet| sheet.saving) {
        return;
    }
    state.generation += 1;
    state.sheet = None;
    state.list_request = None;
    state.save_request = None;
}

/// `markdownSaveFolder`: a new folder re-suggests the name while the name is still the suggested
/// one.
pub fn set_folder(state: &mut SaveMarkdownState, value: &str) {
    let title = state.title.clone();
    let paths = state.paths.clone();
    let Some(sheet) = state.sheet.as_mut().filter(|sheet| !sheet.saving) else {
        return;
    };
    sheet.folder = value.to_string();
    sheet.folder_error = None;
    if sheet.suggested {
        if let Some(paths) = paths {
            sheet.file_name = suggested_markdown_stem(&title, value, &paths);
        }
    }
}

/// `markdownSaveName`: typing a name takes it off the suggestion.
pub fn set_file_name(state: &mut SaveMarkdownState, value: &str) {
    let Some(sheet) = state.sheet.as_mut().filter(|sheet| !sheet.saving) else {
        return;
    };
    sheet.file_name = strip_markdown_extension(value);
    sheet.file_name_error = None;
    sheet.suggested = false;
}

/// `markdownSaveSubmit`: validate, then write the file.
pub fn submit(state: &mut SaveMarkdownState, request_id: u64) -> Vec<Effect> {
    let Some(sheet) = state.sheet.as_ref().filter(|sheet| !sheet.saving) else {
        return Vec::new();
    };
    let folder_error = folder_path_error(&sheet.folder);
    let file_name_error = markdown_stem_error(&sheet.file_name);
    let path = format!(
        "docs/{}/{}.md",
        normalized_folder_path(&sheet.folder),
        normalized_markdown_stem(&sheet.file_name)
    );
    let refused = folder_error.is_some() || file_name_error.is_some() || state.paths.is_none();
    if let Some(sheet) = state.sheet.as_mut() {
        sheet.folder_error = folder_error;
        sheet.file_name_error = file_name_error;
        if !refused {
            sheet.saving = true;
            sheet.file_name_error = None;
        }
    }
    if refused {
        return Vec::new();
    }
    let correlation = correlation_id(state, "save-message-markdown");
    let content = state.markdown.clone();
    state.save_request = Some(SaveMarkdownRequest {
        request_id,
        generation: state.generation,
        correlation: correlation.clone(),
        path: path.clone(),
        stage: SaveMarkdownStage::Save,
    });
    vec![docs_call(
        request_id,
        json!({
            "action": "save",
            "content": content,
            "path": path,
            "requestId": correlation,
        }),
    )]
}

/// Takes the answer to one of the sheet's Docs calls, or `None` when the id is somebody else's.
pub fn settle_rpc(
    state: &mut SaveMarkdownState,
    request_id: u64,
    outcome: Result<&Value, String>,
    next_request: u64,
) -> Option<Vec<Effect>> {
    if let Some((listing_id, generation)) = state.list_request {
        if listing_id == request_id {
            state.list_request = None;
            if generation == state.generation {
                settle_listing(state, outcome);
            }
            return Some(Vec::new());
        }
    }
    let pending = state.save_request.clone()?;
    if pending.request_id != request_id {
        return None;
    }
    state.save_request = None;
    if pending.generation != state.generation {
        return Some(Vec::new());
    }
    Some(settle_save(state, &pending, outcome, next_request))
}

/// `list()` answering: the project's Markdown paths, or the refusal to show under the fields.
fn settle_listing(state: &mut SaveMarkdownState, outcome: Result<&Value, String>) {
    let title = state.title.clone();
    match outcome.and_then(checked_response) {
        Ok(response) => {
            let paths = markdown_paths(&response);
            let folder = state
                .sheet
                .as_ref()
                .map(|sheet| sheet.folder.clone())
                .unwrap_or_default();
            let suggested = state.sheet.as_ref().is_some_and(|sheet| sheet.suggested);
            state.paths = Some(paths.clone());
            if let Some(sheet) = state.sheet.as_mut() {
                sheet.loading = false;
                if suggested {
                    sheet.file_name = suggested_markdown_stem(&title, &folder, &paths);
                }
            }
        }
        Err(message) => {
            if let Some(sheet) = state.sheet.as_mut() {
                sheet.loading = false;
                sheet.listing_error = Some(if message.is_empty() {
                    "Could not read the project's files.".to_string()
                } else {
                    message
                });
            }
        }
    }
}

/// `save()` answering, in its two stages: the write, then the absolute path the host copies.
fn settle_save(
    state: &mut SaveMarkdownState,
    pending: &SaveMarkdownRequest,
    outcome: Result<&Value, String>,
    next_request: u64,
) -> Vec<Effect> {
    let result = match outcome.and_then(checked_response) {
        Ok(response) => response,
        Err(message) => return fail_save(state, message),
    };
    if result.get("requestId").and_then(Value::as_str) != Some(pending.correlation.as_str()) {
        return fail_save(
            state,
            "The Files service returned an invalid response.".to_string(),
        );
    }
    match pending.stage {
        SaveMarkdownStage::Save => {
            if !result.get("file").is_some_and(|file| file.is_object()) {
                return fail_save(
                    state,
                    "The Files service did not return the saved Markdown file.".to_string(),
                );
            }
            let correlation = correlation_id(state, "saved-message-path");
            let path = pending.path.clone();
            state.save_request = Some(SaveMarkdownRequest {
                request_id: next_request,
                generation: state.generation,
                correlation: correlation.clone(),
                path: path.clone(),
                stage: SaveMarkdownStage::ResolvePath,
            });
            vec![docs_call(
                next_request,
                json!({
                    "action": "copyFullPath",
                    "path": path,
                    "requestId": correlation,
                }),
            )]
        }
        SaveMarkdownStage::ResolvePath => {
            let full_path = result
                .get("fullPath")
                .and_then(Value::as_str)
                .filter(|path| !path.is_empty());
            let Some(full_path) = full_path else {
                return fail_save(
                    state,
                    "The Files service did not return the saved Markdown path.".to_string(),
                );
            };
            // The host copies the path and says so; the sheet then closes as it would have.
            let saved = Effect::MarkdownSaved {
                path: full_path.to_string(),
            };
            if let Some(sheet) = state.sheet.as_mut() {
                sheet.saving = false;
            }
            close(state);
            vec![saved]
        }
    }
}

/// The save failing, which leaves the sheet open with the reason under the file name.
fn fail_save(state: &mut SaveMarkdownState, message: String) -> Vec<Effect> {
    if let Some(sheet) = state.sheet.as_mut() {
        sheet.saving = false;
        sheet.file_name_error = Some(if message.is_empty() {
            "Could not save the Markdown file.".to_string()
        } else {
            message
        });
    }
    Vec::new()
}

/// `getSnapshot`: the `saveMarkdown` document value, or `null` when the sheet is closed.
pub fn project(state: &SaveMarkdownState) -> Value {
    let Some(sheet) = state.sheet.as_ref() else {
        return Value::Null;
    };
    let mut value = Map::new();
    value.insert("folder".to_string(), Value::String(sheet.folder.clone()));
    value.insert(
        "fileName".to_string(),
        Value::String(sheet.file_name.clone()),
    );
    value.insert("suggested".to_string(), Value::Bool(sheet.suggested));
    value.insert("loading".to_string(), Value::Bool(sheet.loading));
    value.insert("saving".to_string(), Value::Bool(sheet.saving));
    // The three errors are `undefined` until something fails, which `JSON.stringify` leaves out.
    for (key, error) in [
        ("folderError", &sheet.folder_error),
        ("fileNameError", &sheet.file_name_error),
        ("listingError", &sheet.listing_error),
    ] {
        if let Some(error) = error {
            value.insert(key.to_string(), Value::String(error.clone()));
        }
    }
    Value::Object(value)
}

/// `checkedProjectDocsResponse` minus the id check, which each caller makes against its own.
fn checked_response(value: &Value) -> Result<Map<String, Value>, String> {
    let Some(record) = value.as_object() else {
        return Err("The Files service returned an invalid response.".to_string());
    };
    if let Some(error) = record.get("error").and_then(Value::as_str) {
        if !error.is_empty() {
            return Err(error.to_string());
        }
    }
    Ok(record.clone())
}

/// `listProjectMarkdownDocumentPaths`: the `.md` files of the project's Docs tree.
fn markdown_paths(response: &Map<String, Value>) -> Vec<String> {
    response
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry.get("kind").and_then(Value::as_str) == Some("file"))
                .filter_map(|entry| entry.get("path").and_then(Value::as_str))
                .filter(|path| path.to_lowercase().ends_with(".md"))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `createProjectDocsRequestId`, without the `crypto.randomUUID()` the core may not call.
///
/// The id only has to correlate one answer with one call, and
/// `docs/2026-09-21/rust-chat/SEAM.md` section 7 rule 4 allows a monotonic counter for exactly
/// that. It never reaches the document.
fn correlation_id(state: &mut SaveMarkdownState, prefix: &str) -> String {
    state.docs_request_seq += 1;
    format!("{prefix}-{}", state.docs_request_seq)
}

/// `POST /api/runProjectDocsAction`. The host adds `projectId`, which it already owns.
fn docs_call(request_id: u64, params: Value) -> Effect {
    Effect::SendRpc {
        request_id,
        method: ChatRpcMethod::RunProjectDocsAction,
        params: Box::new(params),
    }
}

/// `.replace(/\.md$/iu, '')` on the typed name, which keeps the surrounding space the field has.
fn strip_markdown_extension(value: &str) -> String {
    let start = match value.len().checked_sub(3) {
        Some(start) if value.is_char_boundary(start) => start,
        _ => return value.to_string(),
    };
    if value[start..].to_lowercase() == ".md" {
        value[..start].to_string()
    } else {
        value.to_string()
    }
}
