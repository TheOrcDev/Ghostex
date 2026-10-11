pub(super) fn tagged_text<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let rest = &text[start..];
    let end = rest.find(&close)?;
    Some(rest[..end].trim())
}

/*
Agent transcripts frequently encode injected instructions as user-role records.
This picker is specifically a history of prompts a person sent, so reject those
provider envelopes at ingestion rather than making every UI hide them later.
*/
const INJECTED_PROVIDER_TAG_NAMES: &[&str] = &[
    "INSTRUCTIONS",
    "agent-message",
    "app-context",
    "bash-input",
    "bash-stderr",
    "bash-stdout",
    "collaboration_mode",
    "command-args",
    "command-message",
    "command-name",
    "cross-session-message",
    "environment_context",
    "fork-boilerplate",
    "ide_opened_file",
    "local-command-caveat",
    "local-command-stderr",
    "local-command-stdout",
    "mcp-polling-update",
    "mcp-resource-update",
    "rules",
    "system-reminder",
    "task-notification",
    "teammate-message",
    "turn_aborted",
    "user-memory-input",
    "user-prompt-submit-hook",
    "user_info",
    "user_instructions",
];

fn leading_tag_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('<')?;
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '>')
        .unwrap_or(rest.len());
    let name = &rest[..end];
    (!name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
    .then_some(name)
}

fn starts_with_injected_provider_envelope(text: &str) -> bool {
    leading_tag_name(text).is_some_and(|tag| INJECTED_PROVIDER_TAG_NAMES.contains(&tag))
        || text.starts_with("<channel source=")
        || text.starts_with("<permissions instructions>")
}

/*
Headless helper runs and harness scaffolding turns land in the same transcripts
as real conversations: Ghostex spawns an agent to write a session title or a
commit message, and agent harnesses inject their own continuation turns. Every
one of those replays the same instruction block, so the picker fills up with
rows that read identically. Reject them by their opening line for the same
reason as the tag envelopes above.
*/
const INJECTED_PROMPT_PREFIXES: &[&str] = &[
    "# AGENTS.md instructions",
    "[Image extracted from tool result above]",
    "[Request interrupted",
    "Briefly inform the user about the task result and perform any follow-up actions",
    "Caveat: The messages below",
    "Continue the conversation using Cursor SDK capabilities only.",
    "Cursor SDK tool boundary:",
    "The beginning of the above subagent result is already visible to the user.",
    "The user interrupted the previous turn:",
    "Write a Git commit message for the staged changes.",
    "Write a concise session title that summarizes the user's text.",
    "You write concise git commit messages.",
];

pub(super) fn visible_user_prompt(text: &str) -> Option<String> {
    let trimmed = text.trim_matches(|ch: char| ch.is_whitespace() || ch.is_control());
    if trimmed.is_empty()
        || starts_with_injected_provider_envelope(trimmed)
        || INJECTED_PROMPT_PREFIXES
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
    {
        return None;
    }
    if let Some(first_token) = trimmed.split_whitespace().next() {
        let is_slash_command = first_token.starts_with('/')
            && first_token
                .chars()
                .skip(1)
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            && !trimmed.contains('\n');
        if is_slash_command {
            return None;
        }
    }
    Some(trimmed.to_string())
}
