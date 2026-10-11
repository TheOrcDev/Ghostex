use super::*;

// ---------------------------------------------------------------------------
// Markdown rendering
// ---------------------------------------------------------------------------

const UNTITLED_EXPORT_TITLE: &str = "Untitled session";
const TITLE_MAX_CHARS: usize = 80;

pub(super) fn export_title(session_title: Option<&str>, transcript: &ParsedTranscript) -> String {
    if let Some(title) = session_title
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return clamp_title(title);
    }
    if let Some(title) = transcript.meta.title.as_deref() {
        return clamp_title(title);
    }
    let first_prompt = transcript
        .entries
        .iter()
        .find(|entry| entry.section == TranscriptExportSection::UserMessage)
        .map(|entry| entry.text.as_str())
        .unwrap_or_default();
    let first_line = first_prompt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if first_line.is_empty() {
        return UNTITLED_EXPORT_TITLE.to_string();
    }
    clamp_title(first_line)
}

fn clamp_title(title: &str) -> String {
    let normalized = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= TITLE_MAX_CHARS {
        return normalized;
    }
    let clipped: String = normalized.chars().take(TITLE_MAX_CHARS).collect();
    format!("{clipped}…")
}

/*
Message bodies are emitted verbatim, headings and all, so agent prose that
contains its own `##` sits at the same outline level as the structural
`## 👤 User` / `## 🤖 Agent` headings. That is deliberate: demoting either side
means rewriting message text, and a leading `#` inside a fenced block is a shell
comment, not a heading, so the rewrite would corrupt content to tidy an outline
no one navigates — this file is read by the next agent, not skimmed in a TOC.
The reference exporter (`codex-md.py`) makes the same call.
*/
pub(super) fn render_markdown(
    title: &str,
    transcript: &ParsedTranscript,
    selection: &SessionTranscriptExportSelection,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {title}\n\n"));
    if selection.includes(TranscriptExportSection::SessionMeta) {
        out.push_str(&render_session_meta(title, &transcript.meta));
    }
    /*
    Patch lines are buffered instead of written per entry: a turn that edits one
    file seven times in a row produced seven near-identical `🔧 path` lines, and
    the next agent has to read past all of them to reach the next real step.
    Consecutive changes to the same file merge into one line with summed counts;
    the first entry that renders anything else — a failure note included — flushes
    the run, so a failure always stays attached to the patch that failed.
    */
    let mut patch_run: Vec<PatchFileChange> = Vec::new();
    for entry in &transcript.entries {
        if !selection.includes(entry.section) {
            continue;
        }
        if !matches!(
            entry.section,
            TranscriptExportSection::Patch | TranscriptExportSection::PatchOutput
        ) {
            flush_patch_run(&mut out, &mut patch_run, transcript.meta.cwd.as_deref());
        }
        match entry.section {
            TranscriptExportSection::SessionMeta => {}
            TranscriptExportSection::UserMessage => {
                out.push_str(&format!("## 👤 User\n\n{}\n\n", entry.text.trim_end()));
            }
            TranscriptExportSection::AgentMessage => {
                out.push_str(&format!("## 🤖 Agent\n\n{}\n\n", entry.text.trim_end()));
            }
            TranscriptExportSection::AgentReasoning => {
                out.push_str(&format!(
                    "> 🧠 **Reasoning:** {}\n\n",
                    one_line(&entry.text)
                ));
            }
            TranscriptExportSection::InternalReasoning => {
                out.push_str(&format!(
                    "> 🔒 **Internal reasoning:** {}\n\n",
                    one_line(&entry.text)
                ));
            }
            TranscriptExportSection::TerminalCmd => {
                out.push_str(&render_tool_call("💻", entry, "bash"));
            }
            TranscriptExportSection::McpCall => {
                out.push_str(&render_tool_call("🔌", entry, "json"));
            }
            TranscriptExportSection::OtherTool => {
                out.push_str(&render_tool_call("🧩", entry, "json"));
            }
            TranscriptExportSection::TerminalOutput
            | TranscriptExportSection::McpOutput
            | TranscriptExportSection::OtherToolOutput => {
                out.push_str(&render_output(entry, selection.terminal_output_tail_lines));
            }
            TranscriptExportSection::Patch => {
                if entry.patch.is_empty() || entry.patch_failure.is_some() {
                    // A failed (or file-less) patch prints on its own, so the
                    // run in front of it has to land first or the summaries
                    // would appear after the patch that follows them.
                    flush_patch_run(&mut out, &mut patch_run, transcript.meta.cwd.as_deref());
                    out.push_str(&render_patch(entry, transcript.meta.cwd.as_deref()));
                } else {
                    for change in &entry.patch {
                        merge_patch_change(&mut patch_run, change);
                    }
                }
            }
            TranscriptExportSection::PatchOutput => {
                // Patch results are folded into the patch summary (plan Q4).
            }
            TranscriptExportSection::WebSearch => out.push_str("> 🔍 **Web search**\n\n"),
            TranscriptExportSection::TokenCount => {
                out.push_str(&format!("> 📊 **Tokens:** {}\n\n", one_line(&entry.text)));
            }
            TranscriptExportSection::TurnContext => {
                out.push_str(&format!("> 🔄 **Turn:** {}\n\n", one_line(&entry.text)));
            }
            TranscriptExportSection::TaskEvent => {
                out.push_str(&format!("> 📌 **{}**\n\n", one_line(&entry.text)));
            }
            TranscriptExportSection::SystemMessage => {
                out.push_str(&format!(
                    "### ⚙️ System message\n\n{}\n\n",
                    fenced_block(&entry.text, "text")
                ));
            }
            TranscriptExportSection::GitSnapshot => {
                out.push_str(&format!("> 📸 **Snapshot:** {}\n\n", one_line(&entry.text)));
            }
            TranscriptExportSection::SessionEvent => {
                out.push_str(&format!("> 🔔 **{}**\n\n", one_line(&entry.text)));
            }
        }
    }
    flush_patch_run(&mut out, &mut patch_run, transcript.meta.cwd.as_deref());
    while out.ends_with("\n\n\n") {
        out.pop();
    }
    out
}

fn flush_patch_run(out: &mut String, run: &mut Vec<PatchFileChange>, cwd: Option<&str>) {
    for change in run.drain(..) {
        out.push_str(&format!("🔧 {}\n\n", patch_change_line(&change, cwd)));
    }
}

/// Folds a change into the open run when it touches the same file as the line
/// currently being built. Only the LAST line is a merge candidate: alternating
/// edits to two files stay in the order the agent made them.
fn merge_patch_change(run: &mut Vec<PatchFileChange>, change: &PatchFileChange) {
    let Some(open) = run.last_mut().filter(|open| open.path == change.path) else {
        run.push(change.clone());
        return;
    };
    open.added += change.added;
    open.removed += change.removed;
    open.start_line = min_line(open.start_line, change.start_line);
    open.end_line = max_line(open.end_line, change.end_line);
    open.kind = merged_patch_kind(open.kind, change.kind);
}

/// A file created and then edited is still a new file; a file whose last change
/// deletes it is deleted, whatever happened to it before.
fn merged_patch_kind(open: PatchChangeKind, next: PatchChangeKind) -> PatchChangeKind {
    match (open, next) {
        (_, PatchChangeKind::Deleted) => PatchChangeKind::Deleted,
        (PatchChangeKind::Added, _) | (_, PatchChangeKind::Added) => PatchChangeKind::Added,
        _ => PatchChangeKind::Updated,
    }
}

fn min_line(open: Option<usize>, next: Option<usize>) -> Option<usize> {
    match (open, next) {
        (Some(open), Some(next)) => Some(open.min(next)),
        (value, None) | (None, value) => value,
    }
}

fn max_line(open: Option<usize>, next: Option<usize>) -> Option<usize> {
    match (open, next) {
        (Some(open), Some(next)) => Some(open.max(next)),
        (value, None) | (None, value) => value,
    }
}

fn render_session_meta(title: &str, meta: &TranscriptMeta) -> String {
    let mut lines: Vec<String> = vec![
        format!("Title: {title}"),
        format!("Agent: {}", agent_display_name(meta.agent)),
    ];
    if let Some(model) = &meta.model {
        lines.push(format!("Model: {model}"));
    }
    if let Some(session_id) = &meta.agent_session_id {
        lines.push(format!("Session: {session_id}"));
    }
    if let Some(cwd) = &meta.cwd {
        lines.push(format!("CWD: {cwd}"));
    }
    if let Some(started_at) = &meta.started_at {
        lines.push(format!("Date: {started_at}"));
    }
    lines.push(format!("Source: {}", meta.source_path.display()));
    format!("```yaml\n{}\n```\n\n", lines.join("\n"))
}

fn render_tool_call(emoji: &str, entry: &ExportEntry, language: &str) -> String {
    let name = entry.tool_name.as_deref().unwrap_or("tool");
    if entry.text.trim().is_empty() {
        return format!("### {emoji} `{name}`\n\n");
    }
    format!(
        "### {emoji} `{name}`\n\n{}\n\n",
        fenced_block(entry.text.trim_end(), language)
    )
}

fn render_output(entry: &ExportEntry, tail_lines: usize) -> String {
    let text = tail_capped(entry.text.trim_end(), tail_lines);
    if text.trim().is_empty() {
        return String::new();
    }
    let label = if entry.is_error { "Error" } else { "Output" };
    format!("**{label}:**\n\n{}\n\n", fenced_block(&text, "text"))
}

/// Keeps the LAST `tail_lines` lines of a block, announcing what was dropped.
fn tail_capped(text: &str, tail_lines: usize) -> String {
    if tail_lines == 0 {
        return text.to_string();
    }
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() <= tail_lines {
        return text.to_string();
    }
    let trimmed = lines.len() - tail_lines;
    let kept = lines[lines.len() - tail_lines..].join("\n");
    format!("... ({trimmed} lines trimmed) ...\n{kept}")
}

fn render_patch(entry: &ExportEntry, cwd: Option<&str>) -> String {
    let mut out = String::new();
    if entry.patch.is_empty() {
        let name = entry.tool_name.as_deref().unwrap_or("patch");
        out.push_str(&format!("🔧 `{name}` (no file changes recorded)\n\n"));
    }
    for change in &entry.patch {
        out.push_str(&format!("🔧 {}\n\n", patch_change_line(change, cwd)));
    }
    if let Some(failure) = &entry.patch_failure {
        out.push_str(&format!("⚠ patch failed: {failure}\n\n"));
    }
    out
}

/// Absolute paths inside the session's own working directory are shown
/// relative to it — the next agent runs there, and the full prefix repeated on
/// every patch line buries the part that matters.
fn project_relative_path(path: &str, cwd: Option<&str>) -> String {
    let Some(cwd) = cwd
        .map(|cwd| cwd.trim_end_matches('/'))
        .filter(|cwd| !cwd.is_empty())
    else {
        return path.to_string();
    };
    match path.strip_prefix(&format!("{cwd}/")) {
        Some(relative) if !relative.is_empty() => relative.to_string(),
        _ => path.to_string(),
    }
}

fn patch_change_line(change: &PatchFileChange, cwd: Option<&str>) -> String {
    let mut line = project_relative_path(&change.path, cwd);
    if let (Some(start), Some(end)) = (change.start_line, change.end_line) {
        if start == end {
            line.push_str(&format!(":{start}"));
        } else {
            line.push_str(&format!(":{start}-{end}"));
        }
    }
    match change.kind {
        PatchChangeKind::Added => line.push_str(" (new file)"),
        PatchChangeKind::Deleted => line.push_str(" (deleted)"),
        PatchChangeKind::Updated => {
            line.push_str(&format!(" (+{}/-{})", change.added, change.removed));
        }
    }
    line
}

pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Picks a fence longer than any backtick run inside the body, so transcript
/// content that itself contains fenced code cannot break out of the block.
fn fenced_block(text: &str, language: &str) -> String {
    let mut longest_run = 0usize;
    let mut current_run = 0usize;
    for character in text.chars() {
        if character == '`' {
            current_run += 1;
            longest_run = longest_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    let fence = "`".repeat(longest_run.max(2) + 1);
    format!("{fence}{language}\n{text}\n{fence}")
}
