use super::*;
use std::path::Path;

/// The part of a Claude agent command that reaches the CLI itself: `claude`, or `cswap run <slot> [cswap flags] --`. `None` for an alias or wrapper Ghostex cannot see through.
fn claude_cli_prefix(agent_command: &str) -> Option<&str> {
    let mut offset = 0;
    while let Some((_, end, word)) = command_word(agent_command, offset) {
        offset = end;
        if word.contains('=') || matches!(word.as_str(), "env" | "exec" | "command") {
            continue;
        }
        let executable = Path::new(&word).file_name()?.to_str()?;
        return match executable {
            "claude" => Some(&agent_command[..end]),
            "cswap" => {
                let (_, run_end, run) = command_word(agent_command, offset)?;
                if run != "run" {
                    return None;
                }
                let (_, mut offset, _) = command_word(agent_command, run_end)?;
                while let Some((_, end, word)) = command_word(agent_command, offset) {
                    if word == "--" {
                        return Some(&agent_command[..end]);
                    }
                    offset = end;
                }
                None
            }
            _ => None,
        };
    }
    None
}

/// CDXC:AgentProviders 2026-09-21 WHY:
/// A Claude session sent to the background (`/bg`, `claude --bg`) keeps running under Claude's daemon after its pane is gone, and `claude --resume <id>` refuses a session id that is still live ("running as a background session"), so the restored pane ended at a shell prompt while the chat still showed the agent working. The restore asks `claude agents --json` first and opens the live session with `claude attach`; only a session that is not running in the background is resumed. Subcommands must follow the executable directly: after the launch flags Claude reads `attach` as a prompt and starts a new conversation.
pub(crate) fn build_claude_attach_or_resume_command(
    agent_command: &str,
    session_id: &str,
    resume_invocation: String,
) -> String {
    let Some(cli) = claude_cli_prefix(agent_command) else {
        return resume_invocation;
    };
    let session_match = quote_shell_arg(&format!("\"sessionId\":\"{session_id}\""));
    format!(
        "__ghostex_claude_bg_id=\"$({cli} agents --json 2>/dev/null | tr -d ' \\n' | tr '}}' '\\n' | grep -F '\"kind\":\"background\"' | grep -F {session_match} | sed -n 's/.*\"id\":\"\\([^\"]*\\)\".*/\\1/p' | head -n 1)\"; if [ -n \"$__ghostex_claude_bg_id\" ]; then {cli} attach \"$__ghostex_claude_bg_id\"; else {resume_invocation}; fi"
    )
}
