use super::*;

pub(crate) fn schedule_agent_title_metadata_check(
    state: AppState,
    project_id: String,
    session_id: String,
) {
    /*
    CDXC:SessionTitles 2026-06-21-15:35:
    Agent CLI renames are accepted asynchronously after Ghostex submits `/rename`. Match TypeScript gxserver's three-second trailing metadata check so Rust promotes the agent's own session-metadata title (Codex `thread_name`, Claude `custom-title`) and broadcasts a presentation delta after the CLI writes it.
    */
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(
            GXSERVER_AGENT_TITLE_METADATA_DEBOUNCE_MS,
        ))
        .await;
        let Ok(db) =
            open_gxserver_database_with_busy_timeout(&state.paths, Duration::from_secs(10))
        else {
            return;
        };
        let repository = DomainRepository::new(&db, state.metadata.server_id.as_str());
        let Ok(changed) = reconcile_agent_metadata_title_for_session(
            &repository,
            &project_id,
            &session_id,
            &state.paths.home_dir,
            "metadata-mismatch",
        ) else {
            return;
        };
        if changed {
            let _ = schedule_presentation_session_delta(
                &state,
                &db,
                &repository,
                &project_id,
                &session_id,
            );
        }
        if let Ok(Some(session)) = repository.get_session(&project_id, &session_id) {
            if let Some(target) = fork_initial_rename_retry_target(&session) {
                schedule_fork_initial_rename(state.clone(), target);
            }
        }
    });
}

pub(crate) fn schedule_agent_title_metadata_checks_for_sessions(
    state: &AppState,
    sessions: &[Value],
) {
    for session in sessions {
        if !should_check_agent_metadata_title_for_project_status(session) {
            continue;
        }
        let Some(project_id) = read_session_text(session, "projectId") else {
            continue;
        };
        let Some(session_id) = read_session_text(session, "sessionId") else {
            continue;
        };
        schedule_agent_title_metadata_check(state.clone(), project_id, session_id);
    }
}

pub(crate) fn should_check_agent_metadata_title_for_project_status(session: &Value) -> bool {
    if !is_agent_associated_session_for_project_status(session) {
        return false;
    }
    if read_runtime_text(session, "pendingAgentTitleRequestStatus").as_deref() == Some("pending") {
        return true;
    }
    read_runtime_text(session, "titleMetadataSource").as_deref() != Some("agent-metadata")
        && trusted_resume_title_for_project_status(session).is_none()
}

pub(crate) fn is_agent_associated_session_for_project_status(session: &Value) -> bool {
    read_session_text(session, "kind").as_deref() == Some("agent")
        || read_session_text(session, "agentId").is_some()
        || read_runtime_text(session, "agentName").is_some()
        || read_runtime_text(session, "agentId").is_some()
        || read_runtime_text(session, "agentSessionId").is_some()
        || read_runtime_text(session, "agentSessionPath").is_some()
}

pub(crate) fn trusted_resume_title_for_project_status(session: &Value) -> Option<String> {
    let title = read_session_text(session, "title")?;
    let title_source = normalize_project_status_title_source(
        read_runtime_text(session, "titleSource")
            .or_else(|| read_runtime_text(session, "restoreTitleSource"))
            .as_deref(),
        &title,
    );
    if title_source == "placeholder" || is_terminal_auto_working_directory_title(session) {
        return None;
    }
    let visible = get_visible_terminal_title(&title)?;
    (!is_rejected_project_status_resume_title(&visible)).then_some(visible)
}

pub(crate) fn normalize_project_status_title_source(
    value: Option<&str>,
    title: &str,
) -> &'static str {
    match value {
        Some("browser-auto") => "browser-auto",
        Some("generated") => "generated",
        Some("placeholder") => "placeholder",
        Some("terminal-auto") => "terminal-auto",
        Some("user") => "user",
        _ if is_temporary_project_status_title(title) => "placeholder",
        _ => "user",
    }
}

pub(crate) fn is_rejected_project_status_resume_title(title: &str) -> bool {
    let normalized = title.trim();
    let lower = normalized.to_ascii_lowercase();
    normalized == "ð^ß^Ñ»"
        || is_temporary_project_status_title(normalized)
        || normalized.starts_with('ð') && normalized.ends_with('»')
        || is_gxserver_session_id(normalized)
        || normalized.chars().any(char::is_control)
        || lower.starts_with("codex ")
        || lower.starts_with("claude ")
        || lower.starts_with("cursor-agent ")
        || lower.starts_with("opencode ")
}

pub(crate) fn is_temporary_project_status_title(title: &str) -> bool {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .eq_ignore_ascii_case("search by text")
}
