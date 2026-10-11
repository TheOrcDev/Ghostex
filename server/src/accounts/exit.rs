use std::{
    path::Path,
    time::{Duration, Instant},
};

use crate::session_chat_send::{
    write_session_chat_payload, SessionChatSendError, SessionChatSendFailure,
    SESSION_CHAT_INTERRUPT,
};

/// Account switching deliberately cancels the old agent regardless of its screen.
/// Each interrupt is a separate stdin write; consecutive Ctrl+C presses must land inside the CLI's double-interrupt window.
///
/// CDXC:AgentProviders 2026-09-29 WHY:
/// With a background shell or agent running, Claude's second Ctrl+C opens an "Exit and stop tasks / Move to background and exit / Stay" dialog that needs two more Ctrl+C presses to exit (stopping the tasks). The old three-Ctrl+C cycle sent its Escape right after the third press, cancelling that dialog every round, so the switch looped until it timed out. Each round therefore sends four Ctrl+C presses before its Escape.
pub(crate) async fn interrupt_until_exited(
    home: &Path,
    project_id: &str,
    session_id: &str,
    zmx_name: &str,
    source: &str,
    timeout_ms: u64,
    cancelled: &(dyn Fn() -> bool + Send + Sync),
) -> Result<(), SessionChatSendError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let keys = [SESSION_CHAT_INTERRUPT, "\u{3}", "\u{3}", "\u{3}", "\u{3}"];
    let mut next_key = 0;
    let mut shell_ready_since = None;
    loop {
        if cancelled() {
            return Err(error(
                SessionChatSendFailure::NotAttempted,
                "The account switch was cancelled.",
            ));
        }
        let name = zmx_name.to_string();
        let home = home.to_path_buf();
        let running = tokio::task::spawn_blocking(move || {
            crate::zmx::read_zmx_session_process_identities(std::slice::from_ref(&name), &home)
                .map_err(|_| ())
                .and_then(|identities| {
                    let running = identities.contains_key(&name);
                    Ok((running, !running && shell_owns_terminal(&name)?))
                })
        })
        .await;
        let (running, shell_ready) = match running {
            Ok(Ok(observation)) => observation,
            _ => {
                return Err(error(
                    SessionChatSendFailure::ComposerNotReady,
                    "Could not inspect the agent while exiting it. Retry the account switch.",
                ))
            }
        };
        if shell_ready {
            if shell_ready_since.get_or_insert_with(Instant::now).elapsed()
                >= Duration::from_millis(300)
            {
                // An interrupt can straddle the CLI exit and leave escape-sequence
                // bytes in the shell editor. Clear that line before the resume command.
                write_session_chat_payload(project_id, session_id, zmx_name, source, "\u{3}")
                    .await
                    .map_err(|message| error(SessionChatSendFailure::Write, &message))?;
                tokio::time::sleep(Duration::from_millis(150)).await;
                return Ok(());
            }
        } else {
            shell_ready_since = None;
        }
        if Instant::now() >= deadline {
            return Err(error(
                SessionChatSendFailure::ComposerNotReady,
                "The agent did not exit back to the shell, so the resume command was not typed.",
            ));
        }
        if cancelled() {
            return Err(error(
                SessionChatSendFailure::NotAttempted,
                "The account switch was cancelled.",
            ));
        }
        if running {
            write_session_chat_payload(project_id, session_id, zmx_name, source, keys[next_key])
                .await
                .map_err(|message| error(SessionChatSendFailure::Write, &message))?;
            next_key = (next_key + 1) % keys.len();
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

/// CDXC:AgentProviders 2026-09-22 WHY:
/// A missing agent PID can be a gap inside an older shell restore script, with a resume lookup or account wrapper still running. Only an idle foreground shell may receive the new account's command.
/// The process capture must drain stdout while ps runs; waiting for exit before reading fills the pipe on machines with many sessions and falsely times out.
#[cfg(not(windows))]
fn shell_owns_terminal(zmx_name: &str) -> Result<bool, ()> {
    let output = crate::zmx::run_zmx_probe_script(
        "ps -axo pid=,ppid=,pgid=,tpgid=,command=".to_string(),
        crate::zmx::ZmxCommandOptions {
            stdout_limit_bytes: Some(crate::zmx::GXSERVER_ZMX_PROCESS_SNAPSHOT_STDOUT_LIMIT_BYTES),
            timeout_ms: Some(3_000),
            ..Default::default()
        },
    )
    .map_err(|_| ())?;
    if output.exit_code != 0 || output.stdout_truncated {
        return Err(());
    }
    let mut rows = Vec::new();
    let mut identity_rows = String::new();
    for line in output.stdout.lines() {
        let mut fields = line.split_whitespace();
        let mut number = || fields.next().and_then(|value| value.parse::<i64>().ok());
        let (Some(pid), Some(ppid), Some(pgid), Some(tpgid)) =
            (number(), number(), number(), number())
        else {
            return Err(());
        };
        let command = fields.collect::<Vec<_>>().join(" ");
        identity_rows.push_str(&format!("{pid} {ppid} {command}\n"));
        rows.push((pid, ppid, pgid, tpgid, command));
    }
    let root = crate::zmx::find_zmx_daemon_process_id(&identity_rows, zmx_name).ok_or(())?;
    Ok(rows.iter().any(|(pid, ppid, pgid, tpgid, command)| {
        *ppid == root
            && *pgid > 0
            && pgid == tpgid
            && command
                .split_whitespace()
                .next()
                .and_then(|name| Path::new(name).file_name())
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    matches!(
                        name.trim_start_matches('-'),
                        "sh" | "bash" | "zsh" | "fish" | "ksh" | "dash" | "nu"
                    )
                })
            && !rows
                .iter()
                .any(|(other_pid, _, other_group, _, _)| other_pid != pid && other_group == pgid)
    }))
}

// Native Windows has no POSIX foreground process groups. It retains the agent-process exit check above.
#[cfg(windows)]
fn shell_owns_terminal(_: &str) -> Result<bool, ()> {
    Ok(true)
}

fn error(failure: SessionChatSendFailure, message: &str) -> SessionChatSendError {
    SessionChatSendError {
        failure,
        message: message.to_string(),
    }
}
