use crate::model::Agent;
use crate::model::Session;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

pub(super) fn resume_agent_session(session: &Session, accept_all: bool) -> io::Result<()> {
    if session.id.trim().is_empty() {
        println!(
            "ghostex-history: no session id recorded for this {} session",
            session.agent
        );
        return Ok(());
    }

    let argv = resume_argv(session.agent, &session.id, accept_all);
    let project = session.project.trim();
    let project_dir = (!project.is_empty() && Path::new(project).is_dir()).then_some(project);

    if let Some(project_dir) = project_dir {
        println!(
            "\x1b[90m-> resuming {} session {} in {}\x1b[0m",
            session.agent, session.id, project_dir
        );
    } else if project.is_empty() {
        println!(
            "\x1b[90m-> resuming {} session {}\x1b[0m",
            session.agent, session.id
        );
    } else {
        println!(
            "\x1b[90m-> resuming {} session {} (project {} missing - using current dir)\x1b[0m",
            session.agent, session.id, project
        );
    }
    io::stdout().flush()?;

    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(project_dir) = project_dir {
        command.current_dir(project_dir);
    }
    if let Err(error) = command.status() {
        println!("ghostex-history: failed to launch {} ({error})", argv[0]);
        println!("Run manually:");
        if !project.is_empty() {
            println!("  cd {}", shell_quote(project));
        }
        println!("  {}", shell_join(&argv));
    }
    Ok(())
}

pub(super) fn resume_argv(agent: Agent, session: &str, accept_all: bool) -> Vec<String> {
    let session = session.to_string();
    match (agent, accept_all) {
        (Agent::Claude, true) => vec![
            "claude".into(),
            "--dangerously-skip-permissions".into(),
            "--resume".into(),
            session,
        ],
        (Agent::Codex, true) => vec!["codex".into(), "--yolo".into(), "resume".into(), session],
        (Agent::Cursor, true) => vec![
            "cursor-agent".into(),
            "--yolo".into(),
            "--resume".into(),
            session,
        ],
        (Agent::Grok, true) => vec![
            "grok".into(),
            "--permission-mode".into(),
            "bypassPermissions".into(),
            "--resume".into(),
            session,
        ],
        (Agent::Pi, true) | (Agent::Pi, false) => vec!["pi".into(), "--session".into(), session],
        (Agent::Claude, false) => vec!["claude".into(), "--resume".into(), session],
        (Agent::Codex, false) => vec!["codex".into(), "resume".into(), session],
        (Agent::Cursor, false) => vec!["cursor-agent".into(), "--resume".into(), session],
        (Agent::Grok, false) => vec!["grok".into(), "--resume".into(), session],
    }
}

fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch, '-' | '_' | '.' | '/' | ':' | '=' | '+' | ',')
        })
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
