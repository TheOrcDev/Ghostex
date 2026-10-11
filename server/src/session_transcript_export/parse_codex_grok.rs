use super::*;

// ---------------------------------------------------------------------------
// Codex parser
// ---------------------------------------------------------------------------

pub(super) fn parse_codex_record(builder: &mut TranscriptBuilder, record: &Map<String, Value>) {
    let Some(payload) = record_of(record.get("payload")) else {
        return;
    };
    match type_of(record) {
        "session_meta" => {
            builder.meta.agent_session_id = text_field(payload, "session_id")
                .or_else(|| text_field(payload, "id"))
                .or(builder.meta.agent_session_id.take());
            builder.set_meta_cwd(text_field(payload, "cwd"));
            builder.push(ExportEntry::new(
                TranscriptExportSection::SessionMeta,
                "Session started",
            ));
        }
        "turn_context" => {
            builder.set_meta_model(text_field(payload, "model"));
            builder.set_meta_cwd(text_field(payload, "cwd"));
            let model = text_field(payload, "model").unwrap_or_else(|| "?".to_string());
            let effort = text_field(payload, "effort").unwrap_or_else(|| "?".to_string());
            builder.push(ExportEntry::new(
                TranscriptExportSection::TurnContext,
                format!("model={model}, effort={effort}"),
            ));
        }
        "compacted" => builder.push(ExportEntry::new(
            TranscriptExportSection::SessionEvent,
            "Context compacted",
        )),
        "event_msg" => parse_codex_event(builder, payload),
        "response_item" => parse_codex_response_item(builder, payload),
        _ => {}
    }
}

fn parse_codex_event(builder: &mut TranscriptBuilder, payload: &Map<String, Value>) {
    match type_of(payload) {
        "user_message" => {
            builder.push_dialog(
                TranscriptExportSection::UserMessage,
                flatten_text(payload.get("message")),
            );
        }
        "agent_message" => {
            builder.push_dialog(
                TranscriptExportSection::AgentMessage,
                flatten_text(payload.get("message")),
            );
        }
        "agent_reasoning" => {
            let text = flatten_text(payload.get("text"));
            if !text.trim().is_empty() {
                builder.push(ExportEntry::new(
                    TranscriptExportSection::AgentReasoning,
                    text,
                ));
            }
        }
        "token_count" => builder.push(ExportEntry::new(
            TranscriptExportSection::TokenCount,
            codex_token_summary(payload),
        )),
        "task_started" | "task_complete" => builder.push(ExportEntry::new(
            TranscriptExportSection::TaskEvent,
            type_of(payload).replace('_', " "),
        )),
        "context_compacted" | "thread_rolled_back" | "turn_aborted" => {
            builder.push(ExportEntry::new(
                TranscriptExportSection::SessionEvent,
                type_of(payload).replace('_', " "),
            ));
        }
        /*
        Newer Codex builds stopped writing the `user_message`/`agent_message`
        event lane and emit `item_completed` items instead; the two lanes are
        mutually exclusive per file. Tool/reasoning items are skipped here
        because their `response_item` twins are the sole source for those.
        */
        "item_completed" => {
            let Some(item) = record_of(payload.get("item")) else {
                return;
            };
            let section = match type_of(item) {
                "UserMessage" => TranscriptExportSection::UserMessage,
                "AgentMessage" => TranscriptExportSection::AgentMessage,
                _ => return,
            };
            builder.push_dialog(section, flatten_text(item.get("content")));
        }
        _ => {}
    }
}

fn codex_token_summary(payload: &Map<String, Value>) -> String {
    let Some(info) = record_of(payload.get("info")) else {
        return "token count".to_string();
    };
    let usage = record_of(info.get("last_token_usage"))
        .or_else(|| record_of(info.get("total_token_usage")))
        .unwrap_or(info);
    let input = usage
        .get("input_tokens")
        .map(Value::to_string)
        .unwrap_or_else(|| "?".to_string());
    let output = usage
        .get("output_tokens")
        .map(Value::to_string)
        .unwrap_or_else(|| "?".to_string());
    format!("in={input}, out={output}")
}

fn parse_codex_response_item(builder: &mut TranscriptBuilder, payload: &Map<String, Value>) {
    let call_id = text_field(payload, "call_id").or_else(|| text_field(payload, "id"));
    match type_of(payload) {
        "reasoning" => {
            let text = flatten_text(payload.get("text"));
            let text = if text.trim().is_empty() {
                flatten_text(payload.get("summary"))
            } else {
                text
            };
            builder.push(ExportEntry::new(
                TranscriptExportSection::InternalReasoning,
                text,
            ));
        }
        "function_call" | "local_shell_call" | "custom_tool_call" => {
            parse_codex_tool_call(builder, payload, call_id);
        }
        "function_call_output" | "custom_tool_call_output" => {
            let output = payload.get("output");
            let is_error = record_of(output).is_some_and(|record| {
                record.get("success") == Some(&Value::Bool(false)) || bool_field(record, "is_error")
            });
            builder.push_output(
                text_field(payload, "call_id"),
                flatten_text(output),
                is_error,
            );
        }
        "web_search_call" => builder.push_call(
            ExportEntry::new(TranscriptExportSection::WebSearch, String::new())
                .with_tool("web_search", call_id),
        ),
        "tool_search_call" => builder.push_call(
            ExportEntry::new(TranscriptExportSection::OtherTool, String::new())
                .with_tool("tool_search", call_id),
        ),
        "tool_search_output" => {
            builder.push_output(call_id, flatten_text(payload.get("tools")), false);
        }
        "ghost_snapshot" => {
            let commit = record_of(payload.get("ghost_commit"))
                .and_then(|commit| text_field(commit, "id"))
                .unwrap_or_else(|| "?".to_string());
            builder.push(ExportEntry::new(
                TranscriptExportSection::GitSnapshot,
                format!("commit {commit}"),
            ));
        }
        /*
        `message` items are the duplicate twin of the event lane: every visible
        turn is written to both, and the response lane additionally carries the
        harness-injected envelopes (AGENTS.md, environment context, developer
        prompts). Only the developer/system role is taken from here; user and
        assistant text comes from the event lane so no turn is doubled.
        */
        "message" => {
            if matches!(
                payload.get("role").and_then(Value::as_str),
                Some("developer" | "system")
            ) {
                let text = flatten_text(payload.get("content"));
                if !text.trim().is_empty() {
                    builder.push(ExportEntry::new(
                        TranscriptExportSection::SystemMessage,
                        text,
                    ));
                }
            }
        }
        _ => {}
    }
}

fn parse_codex_tool_call(
    builder: &mut TranscriptBuilder,
    payload: &Map<String, Value>,
    call_id: Option<String>,
) {
    let name = text_field(payload, "name").unwrap_or_else(|| "tool".to_string());
    let raw_input = payload
        .get("input")
        .filter(|value| !value.is_null())
        .or_else(|| payload.get("arguments").filter(|value| !value.is_null()))
        .or_else(|| payload.get("action").filter(|value| !value.is_null()));
    let input_text = flatten_text(raw_input);
    let arguments = as_arguments(raw_input);

    /*
    Current Codex funnels every tool through one freeform `exec` call whose
    input is a JavaScript snippet (`await tools.exec_command({cmd:…})`,
    `tools.apply_patch({patch})`, …). Classifying on the call name alone would
    file every patch under Terminal Commands, so the snippet's own content
    decides.
    */
    let mut section = classify_tool(&name);
    if contains_patch_envelope(&input_text) || input_text.contains("tools.apply_patch(") {
        section = TranscriptExportSection::Patch;
    } else if input_text.contains("tools.exec_command(")
        || input_text.contains("tools.write_stdin(")
    {
        section = TranscriptExportSection::TerminalCmd;
    }

    match section {
        TranscriptExportSection::Patch => {
            builder.push_call(
                ExportEntry::new(TranscriptExportSection::Patch, String::new())
                    .with_tool(name, call_id)
                    .with_patch(parse_patch_envelope(&input_text)),
            );
        }
        TranscriptExportSection::TerminalCmd => {
            let command = codex_command_text(&arguments, &input_text);
            builder.push_call(
                ExportEntry::new(TranscriptExportSection::TerminalCmd, command)
                    .with_tool(name, call_id),
            );
        }
        other => {
            let text = if matches!(arguments, Value::Null) {
                input_text
            } else {
                pretty_arguments(&arguments)
            };
            builder.push_call(ExportEntry::new(other, text).with_tool(name, call_id));
        }
    }
}

/// Pulls the shell command out of the three shapes Codex uses: a JSON
/// `{command:[…]}` array, a JSON `{cmd:"…"}` object, and the `cmd:"…"` field of
/// a freeform JavaScript `exec` snippet.
fn codex_command_text(arguments: &Value, input_text: &str) -> String {
    if let Some(record) = arguments.as_object() {
        if let Some(Value::Array(command)) = record.get("command") {
            let parts: Vec<String> = command
                .iter()
                .map(|part| flatten_text(Some(part)))
                .filter(|part| !part.is_empty())
                .collect();
            if !parts.is_empty() {
                return parts.join(" ");
            }
        }
        if let Some(command) = argument_text(arguments, &["cmd", "command", "chars"]) {
            return command;
        }
    }
    // A code-mode script can batch several `tools.exec_command({cmd})` calls; export every command it ran.
    let commands = javascript_string_fields(input_text, "cmd");
    if !commands.is_empty() {
        return commands.join("\n");
    }
    javascript_string_field(input_text, "chars").unwrap_or_else(|| input_text.trim().to_string())
}

pub(crate) fn javascript_string_fields(snippet: &str, field: &str) -> Vec<String> {
    let needle = format!("{field}:");
    let mut values = Vec::new();
    let mut offset = 0;
    while let Some(found) = snippet[offset..].find(&needle) {
        let at = offset + found;
        if let Some(value) = javascript_string_field(&snippet[at..], field) {
            values.push(value);
        }
        offset = at + needle.len();
    }
    values
}

/// Reads `<field>:"…"` out of a JavaScript snippet, honoring backslash escapes
/// so an embedded quote does not end the value early.
fn javascript_string_field(snippet: &str, field: &str) -> Option<String> {
    let needle = format!("{field}:");
    let start = snippet.find(&needle)? + needle.len();
    let rest = snippet[start..].trim_start();
    let mut characters = rest.chars();
    if characters.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Some(unescape_javascript(&value)),
            '\\' => {
                let Some(escaped) = characters.next() else {
                    break;
                };
                value.push('\\');
                value.push(escaped);
            }
            other => value.push(other),
        }
    }
    None
}

fn unescape_javascript(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            text.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => text.push('\n'),
            Some('t') => text.push('\t'),
            Some('r') => {}
            Some(other) => text.push(other),
            None => break,
        }
    }
    text
}

// ---------------------------------------------------------------------------
// Grok parser
// ---------------------------------------------------------------------------

pub(super) fn parse_grok_record(builder: &mut TranscriptBuilder, record: &Map<String, Value>) {
    match type_of(record) {
        "user" => {
            // A synthetic reason marks a harness-injected turn, and the
            // `<user_info>` bootstrap block is the session's system context.
            let text = strip_grok_user_query(&flatten_text(record.get("content")));
            let section = if text_field(record, "synthetic_reason").is_some()
                || text
                    .trim_start()
                    .to_ascii_lowercase()
                    .starts_with("<user_info>")
            {
                TranscriptExportSection::SystemMessage
            } else {
                TranscriptExportSection::UserMessage
            };
            builder.push_dialog(section, text);
        }
        "assistant" => {
            builder.set_meta_model(text_field(record, "model"));
            builder.push_dialog(
                TranscriptExportSection::AgentMessage,
                flatten_text(record.get("content")),
            );
            if let Some(Value::Array(tool_calls)) = record.get("tool_calls") {
                for tool_call in tool_calls {
                    let Some(tool_call) = tool_call.as_object() else {
                        continue;
                    };
                    parse_grok_tool_call(builder, tool_call);
                }
            }
        }
        "reasoning" => {
            let text = flatten_text(record.get("text"));
            let text = if text.trim().is_empty() {
                flatten_text(record.get("summary"))
            } else {
                text
            };
            if !text.trim().is_empty() {
                builder.push(ExportEntry::new(
                    TranscriptExportSection::AgentReasoning,
                    text,
                ));
            }
        }
        "backend_tool_call" | "tool_call" => parse_grok_tool_call(builder, record),
        "tool_result" => {
            let output = record
                .get("content")
                .filter(|value| !value.is_null())
                .or_else(|| record.get("output").filter(|value| !value.is_null()))
                .or_else(|| record.get("result").filter(|value| !value.is_null()));
            builder.push_output(
                text_field(record, "tool_call_id").or_else(|| text_field(record, "id")),
                flatten_text(output),
                bool_field(record, "is_error") || bool_field(record, "isError"),
            );
        }
        "session" | "session_start" => {
            builder.set_meta_cwd(text_field(record, "cwd"));
            builder.set_meta_title(text_field(record, "title"));
            builder.push(ExportEntry::new(
                TranscriptExportSection::SessionMeta,
                "Session started",
            ));
        }
        _ => {}
    }
}

fn parse_grok_tool_call(builder: &mut TranscriptBuilder, record: &Map<String, Value>) {
    let name = record_of(record.get("kind"))
        .and_then(|kind| text_field(kind, "tool_type"))
        .or_else(|| text_field(record, "name"))
        .or_else(|| text_field(record, "tool"))
        .unwrap_or_else(|| "tool".to_string());
    let call_id = text_field(record, "tool_call_id").or_else(|| text_field(record, "id"));
    let arguments = as_arguments(
        record
            .get("arguments")
            .filter(|value| !value.is_null())
            .or_else(|| record.get("input").filter(|value| !value.is_null()))
            .or_else(|| record.get("args").filter(|value| !value.is_null()))
            .or_else(|| record.get("kind").filter(|value| !value.is_null())),
    );
    let command = argument_text(&arguments, &["command", "cmd", "script"]);
    let mut section = classify_tool(&name);
    if section == TranscriptExportSection::TerminalCmd
        && command.as_deref().is_some_and(contains_patch_envelope)
    {
        section = TranscriptExportSection::Patch;
    }
    match section {
        TranscriptExportSection::TerminalCmd => builder.push_call(
            ExportEntry::new(
                TranscriptExportSection::TerminalCmd,
                command.unwrap_or_else(|| pretty_arguments(&arguments)),
            )
            .with_tool(name, call_id),
        ),
        TranscriptExportSection::Patch => {
            let changes = grok_patch_changes(&arguments, command.as_deref());
            builder.push_call(
                ExportEntry::new(TranscriptExportSection::Patch, String::new())
                    .with_tool(name, call_id)
                    .with_patch(changes),
            );
        }
        other => builder.push_call(
            ExportEntry::new(other, pretty_arguments(&arguments)).with_tool(name, call_id),
        ),
    }
}

fn grok_patch_changes(arguments: &Value, command: Option<&str>) -> Vec<PatchFileChange> {
    if let Some(command) = command.filter(|text| contains_patch_envelope(text)) {
        return parse_patch_envelope(command);
    }
    let patch = argument_text(arguments, &["patch", "input", "diff"]);
    if let Some(patch) = patch.filter(|text| contains_patch_envelope(text)) {
        return parse_patch_envelope(&patch);
    }
    let Some(record) = arguments.as_object() else {
        return Vec::new();
    };
    let path = text_field(record, "path")
        .or_else(|| text_field(record, "file_path"))
        .or_else(|| text_field(record, "filename"))
        .unwrap_or_default();
    if path.is_empty() {
        return Vec::new();
    }
    let mut change = new_patch_change(&path, PatchChangeKind::Updated);
    change.removed = line_count(&flatten_text(
        record.get("old_str").or_else(|| record.get("old_string")),
    ));
    change.added = line_count(&flatten_text(
        record
            .get("new_str")
            .or_else(|| record.get("new_string"))
            .or_else(|| record.get("content")),
    ));
    if change.removed == 0 && change.added > 0 {
        change.kind = PatchChangeKind::Added;
    }
    vec![change]
}

pub(super) fn strip_grok_user_query(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let Some(start) = lower.find("<user_query>") else {
        return text.to_string();
    };
    let body_start = start + "<user_query>".len();
    match lower[body_start..].find("</user_query>") {
        Some(end) => text[body_start..body_start + end].trim().to_string(),
        None => text[body_start..].trim().to_string(),
    }
}
