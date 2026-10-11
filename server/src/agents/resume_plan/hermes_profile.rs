use serde_json::{Map, Value};
use std::ops::Range;
use std::path::PathBuf;

use super::{command_word, infer_agent_id_from_command, is_option_word, read_text_from_map};

/// CDXC:Bots 2026-09-28 DECISION:
/// User: resuming a Hermes conversation started outside Ghostex passes `-p <profile>` for the store it came from, so it resumes under the right bot: `-p <profile>` for `HERMES_HOME/profiles/<profile>` and `-p default` for `HERMES_HOME` itself, since plain `hermes` follows the sticky profile `hermes profile use` sets.
/// Supersedes the 2026-09-26 rule that left the default profile's as `{cmd} --resume <id>`.
/// CDXC:Bots 2026-09-28 DECISION:
/// User: a woken Hermes session resumes under the profile it was launched with, replacing a different `-p`/`--profile` in the project's Hermes command (a `hermes -p dobby` session woken in a project whose Hermes agent is `hermes -p harry` had resumed Dobby's conversation under Harry). Supersedes the note that a command already picking a profile is kept as it is.
/// CDXC:Bots 2026-09-29 WHY:
/// The store that holds the conversation decides first (`hermes_session_store`, the lookup chat reads use), since the launch command cannot say which store a plain `hermes` launch wrote to: that follows whatever sticky profile `hermes profile use` had set, and default-bot sessions launched before 2026-09-28 recorded plain `hermes`. The launch command decides only while no store holds the conversation yet.
pub(super) fn with_session_hermes_profile(
    agent_id: Option<&str>,
    mut command: String,
    runtime_settings: &Map<String, Value>,
    launch_settings: &Map<String, Value>,
) -> String {
    if agent_id != Some("hermes-agent") {
        return command;
    }
    let Some(profile) = session_hermes_profile(runtime_settings, launch_settings) else {
        return command;
    };
    let option = profile
        .as_deref()
        .map(|profile| format!(" -p {profile}"))
        .unwrap_or_default();
    match profile_option(&command) {
        Some((_, current)) if profile.as_deref() == Some(current.as_str()) => {}
        Some((span, _)) => command.replace_range(span, &option),
        None => command.push_str(&option),
    }
    command
}

/// The profile a session's conversation belongs to: `Some(None)` for a launch that named none, `None`
/// when nothing the session recorded says, or the name is not a plain word safe to type unquoted.
fn session_hermes_profile(
    runtime_settings: &Map<String, Value>,
    launch_settings: &Map<String, Value>,
) -> Option<Option<String>> {
    let stored_home = || {
        let session_id = read_text_from_map(runtime_settings, "agentSessionId")?;
        let store = crate::session_chat_hermes::hermes_session_store(
            &crate::session_chat_hermes::hermes_home(),
            &session_id,
        )?;
        Some(store.parent()?.to_path_buf())
    };
    let home = read_text_from_map(runtime_settings, "externalAgentHome")
        .map(PathBuf::from)
        .or_else(stored_home);
    let profile = if let Some(home) = home {
        if home.parent()?.file_name()? != "profiles" {
            return Some(Some("default".to_string()));
        }
        home.file_name()?.to_str()?.to_string()
    } else {
        let launch_plan = launch_settings
            .get("agentLaunchPlan")
            .and_then(Value::as_object);
        let command = [
            read_text_from_map(runtime_settings, "agentCommand"),
            launch_plan.and_then(|plan| read_text_from_map(plan, "command")),
            read_text_from_map(launch_settings, "startupText"),
        ]
        .into_iter()
        .flatten()
        .find(|command| infer_agent_id_from_command(command).as_deref() == Some("hermes-agent"))?;
        let Some((_, profile)) = profile_option(&command) else {
            return Some(None);
        };
        profile
    };
    crate::bot_projects::is_bot_profile_name(&profile).then_some(Some(profile))
}

/// The `-p <profile>`, `--profile <profile>` or `--profile=<profile>` of a command's Hermes
/// invocation: its span in the command, with the whitespace before it, and the profile it names.
/// Only words after the `hermes` word count (all of them for a wrapper that has none), up to a
/// shell list operator, so `mkdir -p logs && hermes` names no profile. Hermes reads no attached
/// `-p<profile>`.
fn profile_option(command: &str) -> Option<(Range<usize>, String)> {
    let words: Vec<_> = std::iter::successors(command_word(command, 0), |(_, end, _)| {
        command_word(command, *end)
    })
    .collect();
    let first = words
        .iter()
        .position(|(_, _, word)| {
            infer_agent_id_from_command(word).as_deref() == Some("hermes-agent")
        })
        .map_or(0, |index| index + 1);
    let mut words = words[first..].iter();
    while let Some((start, end, word)) = words.next() {
        let literal = &command[*start..*end];
        if matches!(literal, "&&" | "||" | ";" | "|" | "&") {
            return None;
        }
        if !is_option_word(literal, word) {
            continue;
        }
        let start = command[..*start].trim_end().len();
        if let Some(profile) = word.strip_prefix("--profile=") {
            return Some((start..*end, profile.to_string()));
        }
        if matches!(word.as_str(), "-p" | "--profile") {
            let (_, end, profile) = words.next()?;
            return Some((start..*end, profile.clone()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn an_outside_hermes_session_resumes_under_its_profile() {
        let settings = normalize_agent_settings(None);
        let resume = |project: Value, home: &str| {
            let session = json!({
                "agentId": "hermes-agent",
                "runtimeSettings": {
                    "agentSessionId": "20260926_111001_e55ebd",
                    "externalSession": true,
                    "externalAgentHome": home,
                },
            });
            build_agent_resume_plan(&project, &session, &settings)["primaryCommand"].clone()
        };
        let harry = "/Users/me/.hermes/profiles/harry";
        let project = |launch_settings: Value| json!({ "path": harry, "customAgents": [], "launchSettings": launch_settings });
        let expected = json!("hermes -p harry --resume \"20260926_111001_e55ebd\"");
        assert_eq!(resume(project(json!({})), harry), expected);
        assert_eq!(
            resume(
                project(json!({ "isBot": true, "botProfile": "harry" })),
                harry
            ),
            expected,
            "a bot project's own -p is not repeated"
        );
        assert_eq!(
            resume(
                json!({ "path": "/Users/me/.hermes", "customAgents": [], "launchSettings": {} }),
                "/Users/me/.hermes",
            ),
            json!("hermes -p default --resume \"20260926_111001_e55ebd\""),
        );
    }
}
