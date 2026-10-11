use super::*;

/*
CDXC:SessionChat 2026-08-21:
The sidebar's queued-prompt badge reads `queuedPromptCount` off the presentation
projection, NOT off the chat frame — the sidebar renders every session from that
snapshot and holds no per-session chat subscription. A queue change on an
otherwise idle session produces no other delta, so without this publish the badge
would only appear whenever some unrelated event happened to fire.

CDXC:Drafts 2026-08-28:
`/api/setSessionChatDraft` rides the same publish, which is what keeps a DRAFT
session's sidebar title following the user's typing on every client: the draft
display title is a presentation overlay read from `session_chat_drafts`, so the
delta this schedules is the only thing that republishes it.

Every queue mutation and every scheduler delivery already funnels through
`broadcast_session_chat_queue_state`, so the delta is published there rather than
at each call site. Same shape as `delayed_sends::publish_session_change`: one
short sequencer-locked section that projects, allocates the revision and
broadcasts, so revision order and broadcast order stay identical.
*/
/// Clears a draft session's marker after a chat/queue send reached the agent,
/// and republishes the row so every client stops drawing it as a draft. A send
/// into a session that was never a draft costs one indexed read and no write.
/// The chat half of "Unpark after sending a message" (session_parking.rs).
/// Sits next to draft promotion because it wants the same moment: after the
/// bytes were accepted, and never for an option command.
pub(super) fn unpark_session_after_send(state: &AppState, project_id: &str, session_id: &str) {
    let Ok(db) = open_gxserver_database(&state.paths) else {
        return;
    };
    let repository = DomainRepository::new(&db, state.metadata.server_id.as_str());
    if matches!(
        crate::session_parking::unpark_session_after_user_message(
            &repository,
            &state.paths.app_config_dir,
            project_id,
            session_id,
            &crate::server::now_iso(),
        ),
        Ok(true)
    ) {
        let _ =
            schedule_presentation_session_delta(state, &db, &repository, project_id, session_id);
    }
}

pub(super) fn promote_draft_session_after_send(
    state: &AppState,
    project_id: &str,
    session_id: &str,
) {
    let Ok(db) = open_gxserver_database(&state.paths) else {
        return;
    };
    let repository = DomainRepository::new(&db, state.metadata.server_id.as_str());
    if matches!(
        crate::agents::promote_draft_session(&repository, project_id, session_id),
        Ok(true)
    ) {
        let _ =
            schedule_presentation_session_delta(state, &db, &repository, project_id, session_id);
    }
}

/// Publishes retirement only after a composer send was accepted; queue insertion
/// already retired the draft belonging to a queued prompt.
pub(super) fn retire_sent_session_chat_draft(
    state: &AppState,
    project_id: &str,
    session_id: &str,
    sent_text: &str,
    draft_before_send: Option<&crate::session_chat_queue::SessionChatDraft>,
) {
    let Ok(db) = open_gxserver_database(&state.paths) else {
        return;
    };
    let before = crate::session_chat_queue::read_session_chat_queue_snapshot_with(
        &db, project_id, session_id,
    )
    .draft;
    let result = crate::session_chat_queue::clear_session_chat_draft_after_send(
        &db,
        project_id,
        session_id,
        sent_text,
        draft_before_send,
    );
    crate::session_chat_draft_diagnostics::log(
        &state.logger,
        "serverRetireSettled",
        project_id,
        session_id,
        json!({
            "sent": crate::session_chat_draft_diagnostics::fingerprint(sent_text),
            "captured": draft_before_send.map(|draft| json!({
                "value": crate::session_chat_draft_diagnostics::fingerprint(&draft.content),
                "updatedAt": draft.updated_at, "originClientId": draft.origin_client_id,
            })),
            "previous": before.as_ref().map(|draft| json!({
                "value": crate::session_chat_draft_diagnostics::fingerprint(&draft.content),
                "updatedAt": draft.updated_at, "originClientId": draft.origin_client_id,
                "equalsSent": draft.content.trim() == sent_text.trim(),
                "prefixOfSent": !draft.content.is_empty() && sent_text.starts_with(&draft.content),
            })),
            "cleared": matches!(result, Ok(true)), "completed": result.is_ok(),
        }),
    );
    if matches!(result, Ok(true)) {
        broadcast_session_chat_queue_state(state, project_id, session_id);
    }
}

/// Re-arms a draft's launch activity-suppression window after Ghostex typed one
/// of its OWN commands (a model/effort pick) into the terminal, so the churn
/// that command causes cannot be mistaken for the user prompting the agent. A
/// no-op on every session that is not a draft.
pub(super) fn suppress_draft_activity_after_app_command(
    state: &AppState,
    project_id: &str,
    session_id: &str,
) {
    let Ok(db) = open_gxserver_database(&state.paths) else {
        return;
    };
    let repository = DomainRepository::new(&db, state.metadata.server_id.as_str());
    let Ok(Some(session)) = repository.get_session(project_id, session_id) else {
        return;
    };
    let _ = crate::agents::arm_draft_launch_activity_suppression(&repository, &session);
}

pub(crate) fn publish_session_chat_queue_presentation_delta(
    state: &AppState,
    project_id: &str,
    session_id: &str,
) {
    let Ok(db) = open_gxserver_database(&state.paths) else {
        return;
    };
    let repository = DomainRepository::new(&db, state.metadata.server_id.as_str());
    let _ = schedule_presentation_session_delta(state, &db, &repository, project_id, session_id);
}
