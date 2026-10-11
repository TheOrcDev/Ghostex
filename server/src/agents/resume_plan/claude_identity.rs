use std::path::PathBuf;

use crate::agent_hooks::read_claude_hook_surface_records;
use crate::resume_lookup::expand_home;

fn written_transcript(path: Option<&str>) -> Option<PathBuf> {
    path.map(expand_home).filter(|path| path.is_file())
}

/// CDXC:SessionIdentity 2026-09-24 WHY:
/// A Claude that starts in a pane and never receives a prompt reports a conversation id through its hooks but never writes a transcript, and before the transcript guard such an id replaced the pane's real conversation (session G3aa6 on 2026-09-21). Resuming it fails with "No conversation found", so a stored id with no transcript on disk resumes the newest conversation this pane wrote instead; its next hook event stores the repaired id.
pub(super) fn written_claude_identity(
    surface_id: Option<&str>,
    agent_session_id: Option<String>,
    agent_session_path: Option<String>,
) -> (Option<String>, Option<String>) {
    let (Some(surface_id), Some(stored_id)) = (surface_id, agent_session_id.as_deref()) else {
        return (agent_session_id, agent_session_path);
    };
    if written_transcript(agent_session_path.as_deref()).is_some() {
        return (agent_session_id, agent_session_path);
    }
    let records =
        read_claude_hook_surface_records(&crate::paths::get_gxserver_paths(None), surface_id);
    if records.iter().any(|record| {
        record.agent_session_id == stored_id
            && written_transcript(record.agent_session_path.as_deref()).is_some()
    }) {
        return (agent_session_id, agent_session_path);
    }
    let Some(replacement) = records.iter().rev().find(|record| {
        record.agent_session_id != stored_id
            && written_transcript(record.agent_session_path.as_deref()).is_some()
    }) else {
        return (agent_session_id, agent_session_path);
    };
    if crate::agent_transcripts::find_claude_transcript(stored_id).is_some() {
        return (agent_session_id, agent_session_path);
    }
    (
        Some(replacement.agent_session_id.clone()),
        replacement.agent_session_path.clone(),
    )
}
