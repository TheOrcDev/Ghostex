//! Marked handovers: the style note that asks agents to mark the sections of their final message.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::*;
use crate::paths::{get_gxserver_paths, GxserverPaths};

/// CDXC:MarkedHandovers 2026-10-11 DECISION:
/// User: "Setting, every agent session". With the Marked handovers setting on, every agent session Ghostex starts is told to start each relevant section of the message it hands back to the user with one of these markers, so a skim of the chat shows what needs the user. The agent otherwise writes as it does today. The note goes into every session's prompt, so it stays short and plain.
/// SEE-ALSO: packages/settings-catalog/src/data/defaults.rs (`markedHandovers`, off by default), server/src/coordinators/brief.rs (`thread_brief` repeats it for a thread's report), server/src/agents/activity.rs (ZCode's SessionStart context).
pub const MARKED_HANDOVERS_NOTE: &str = "Marked handovers. The user asked for these markers (use the emoji even where you would otherwise avoid emoji):
❓ Needs you: questions, decisions, anything waiting on the user
✅ Done: finished or pushed
⚠️ Read this: important, a risk, or something that changed
🔄 In progress: still being worked on
🧪 Untested: done but not tried for real yet
🚀 Shipped: released or published to users
🛑 Blocked: stuck until something happens
Rule: in your final message of each turn, the one that hands back to the user, every section starts with the marker that fits it, on a short line or a bold label line, for example:
✅ **Done:** fixed the login check and pushed it.
⚠️ **Read this:** the old token format no longer works.
🧪 **Untested:** the Windows path; I could not run it here.
❓ **Needs you:** keep the old endpoint for one more release?
Use only the markers that apply. Otherwise write as you normally do, and leave the markers out of your other messages.";

const SETTING_KEY: &str = "markedHandovers";

/// Whether the Marked handovers setting is on, read fresh from the shared settings file so a
/// change reaches the next launch or resume without a gxserver restart.
pub(crate) fn marked_handovers_enabled(paths: &GxserverPaths) -> bool {
    // Unit tests assert exact commands, which must not depend on the user's settings.
    if cfg!(test) {
        return false;
    }
    std::fs::read_to_string(paths.app_config_dir.join("native-sidebar-settings.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|settings| settings.get(SETTING_KEY).and_then(Value::as_bool))
        .unwrap_or(false)
}

fn marked_handovers_dir(paths: &GxserverPaths) -> PathBuf {
    paths.root_dir.join("marked-handovers")
}

/// CDXC:MarkedHandovers 2026-10-11 WHY:
/// The note reaches Claude, Codex and ZCode at launch, the way an orchestrator's role does (coordinators/role.rs), but it is added when the launch or resume command is resolved instead of being saved in the session's base command: the command is resolved again on every start, resume, fork and account switch, so turning the setting on or off reaches each session the next time it starts or resumes, while a running agent keeps the instructions it started with. Claude keeps only the last `--append-system-prompt-file` (tested 2026-10-11 on Claude Code 2.1.296), so an orchestrator's role file is swapped for one that carries the role and the note; Codex reads one `developer_instructions`, so an orchestrator's guide pointer and the note share it. A command that already names a system prompt file or developer instructions of its own is left alone rather than lose them. ZCode has no such flag and gets the note as SessionStart hook context (activity.rs); Empryo ignores hook context and other agents have no channel Ghostex can use, so they do not get the note.
/// SEE-ALSO: server/src/agents/launch_plan.rs (`resolve_agent_launch_command` calls this), server/src/coordinators/role.rs (`with_coordinator_role`, the role flags this combines with).
pub(crate) fn with_marked_handovers(agent_id: &str, icon: Option<&str>, command: &str) -> String {
    let paths = get_gxserver_paths(None);
    if command.trim().is_empty() || !marked_handovers_enabled(&paths) {
        return command.to_string();
    }
    let family = default_agent_session_title_name(agent_id)
        .map(|_| agent_id.to_string())
        .or_else(|| icon.and_then(default_agent_icon_to_id).map(str::to_string))
        .or_else(|| infer_agent_id_from_command_executable(command));
    match family.as_deref() {
        Some("claude") => with_claude_note(&paths, command),
        Some("codex") => with_codex_note(command),
        _ => command.to_string(),
    }
}

fn with_claude_note(paths: &GxserverPaths, command: &str) -> String {
    const FLAG: &str = "--append-system-prompt-file";
    let dir = marked_handovers_dir(paths);
    if command.contains(&dir.to_string_lossy().to_string()) {
        return command.to_string();
    }
    if !command.contains(FLAG) {
        let Ok(note) = write_note_file(&dir.join("note.md"), MARKED_HANDOVERS_NOTE) else {
            return command.to_string();
        };
        return format!("{} {FLAG} {}", command.trim_end(), quoted_path(&note));
    }
    let role_file = crate::coordinators::coordinator_role_file(paths);
    let role_flag = format!("{FLAG} {}", quoted_path(&role_file));
    if !command.contains(&role_flag) {
        return command.to_string();
    }
    let combined = format!(
        "{}\n\n{MARKED_HANDOVERS_NOTE}\n",
        crate::coordinators::COORDINATOR_ROLE_PROMPT.trim_end()
    );
    match write_note_file(&dir.join("coordinator-role.md"), &combined) {
        Ok(file) => command.replace(&role_flag, &format!("{FLAG} {}", quoted_path(&file))),
        Err(_) => command.to_string(),
    }
}

fn with_codex_note(command: &str) -> String {
    let pointer_arg = crate::coordinators::codex_developer_instructions_arg(
        crate::coordinators::GUIDE_POINTER_COORDINATOR_INSTRUCTIONS,
    );
    if command.contains(&pointer_arg) {
        let combined = format!(
            "{}\n\n{MARKED_HANDOVERS_NOTE}",
            crate::coordinators::GUIDE_POINTER_COORDINATOR_INSTRUCTIONS
        );
        return command.replace(&pointer_arg, &codex_note_arg(&combined));
    }
    if command.contains("developer_instructions=") {
        return command.to_string();
    }
    format!(
        "{} {}",
        command.trim_end(),
        codex_note_arg(MARKED_HANDOVERS_NOTE)
    )
}

/// `-c developer_instructions="…"` with the value as an ASCII-only TOML basic string: Codex parses
/// a `-c` value as TOML (and takes it as a raw string, quotes included, when that fails), and an
/// ASCII command types into every terminal and shell unchanged.
fn codex_note_arg(text: &str) -> String {
    let mut value = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => value.push_str("\\\""),
            '\\' => value.push_str("\\\\"),
            '\n' => value.push_str("\\n"),
            ' '..='~' => value.push(character),
            _ if (character as u32) <= 0xFFFF => {
                value.push_str(&format!("\\u{:04X}", character as u32))
            }
            _ => value.push_str(&format!("\\U{:08X}", character as u32)),
        }
    }
    value.push('"');
    format!(
        "-c {}",
        quote_shell_arg(&format!("developer_instructions={value}"))
    )
}

fn quoted_path(path: &Path) -> String {
    quote_shell_arg(&path.to_string_lossy())
}

fn write_note_file(path: &Path, content: &str) -> std::io::Result<PathBuf> {
    if std::fs::read_to_string(path).ok().as_deref() != Some(content) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(path.to_path_buf())
}
