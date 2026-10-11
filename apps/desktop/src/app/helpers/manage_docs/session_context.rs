use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use super::*;
use crate::app::helpers::*;

pub(crate) fn manage_docs_action_item<'a>(
    context: ManageDocsContext<'a>,
    path: Option<&str>,
    unavailable_message: &str,
) -> Result<(PathBuf, ManageDocsPath<'a>, fs::Metadata), String> {
    let path = manage_docs_path(context, path)?;
    if path.inner.is_empty() {
        return Err(unavailable_message.to_string());
    }
    let target = manage_operation_url(&path)?;
    manage_validate_docs_action_relative_path(&path, context)?;
    let metadata = fs::metadata(&target).map_err(|_| unavailable_message.to_string())?;
    Ok((target, path, metadata))
}

pub(crate) fn manage_docs_action_item_path(
    context: ManageDocsContext<'_>,
    path: Option<&str>,
    unavailable_message: &str,
) -> Result<String, String> {
    let (target, _, _) = manage_docs_action_item(context, path, unavailable_message)?;
    Ok(target.to_string_lossy().into_owned())
}

pub(crate) fn manage_session_context_prompt(
    context: ManageDocsContext<'_>,
    path: Option<&str>,
) -> Result<String, String> {
    /*
    CDXC:Docs 2026-08-08:
    Session-context staging reads only a validated Docs file, caps it before
    and after the read, rejects binary/non-UTF-8 content, and formats a fenced
    relative-path block. The CEF response strips this private prompt before
    dispatch; only the selected live agent terminal receives it.
    */
    let unavailable = "Select a file to add to session context.";
    let (target, path, metadata) = manage_docs_action_item(context, path, unavailable)?;
    /*
    CDXC:Docs 2026-08-09:
    The prompt names the file the way the Docs tree does — the mount's own name,
    not the reserved routing segment — because this text is read by a human and
    by the agent in the terminal it is pasted into.
    */
    let relative_path = path.display(context);
    if !metadata.is_file() {
        return Err(unavailable.to_string());
    }
    if metadata.len() > MANAGE_SESSION_CONTEXT_MAX_BYTES as u64 {
        return Err("File is too large to add to session context.".to_string());
    }
    let data = fs::read(&target).map_err(|_| unavailable.to_string())?;
    if data.len() > MANAGE_SESSION_CONTEXT_MAX_BYTES {
        return Err("File is too large to add to session context.".to_string());
    }
    if data.contains(&0) {
        return Err("Only UTF-8 text files can be added to session context.".to_string());
    }
    let text = String::from_utf8(data)
        .map_err(|_| "Only UTF-8 text files can be added to session context.".to_string())?;
    let fence = manage_session_context_fence(&text);
    let language = manage_session_context_language(&relative_path);
    let fence_header = if language.is_empty() {
        fence.clone()
    } else {
        format!("{fence}{language}")
    };
    Ok(format!(
        "\nFile context: {relative_path}\n\n{fence_header}\n{text}\n{fence}\n"
    ))
}

pub(crate) fn manage_session_context_fence(text: &str) -> String {
    let mut length = 3;
    while text.contains(&"`".repeat(length)) {
        length += 1;
    }
    "`".repeat(length)
}

pub(crate) fn manage_session_context_language(relative_path: &str) -> &'static str {
    match Path::new(relative_path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("css") => "css",
        Some("excalidraw" | "json") => "json",
        Some("htm" | "html") => "html",
        Some("js" | "mjs") => "javascript",
        Some("md" | "markdown" | "mdown" | "mkdn") => "markdown",
        Some("sh" | "zsh") => "shell",
        Some("swift") => "swift",
        Some("ts" | "tsx") => "typescript",
        _ => "",
    }
}
