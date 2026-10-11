//! What a new snapshot changed, and which overlays it finished.

use ghostex_gx_protocol::LifecycleState;

use super::loaded::{project_id_order, tab_listing, LoadedPresentation};
use super::store::{MachinePresentation, PresentationState};
use crate::change::ChangeSummary;
use crate::keys::{MachineId, ProjectKey, SessionKey};
use crate::overlay::PatchVerdict;

/// Lists what differs between two loaded states of one machine.
pub(super) fn diff_loaded(
    machine: &MachineId,
    previous: &LoadedPresentation,
    next: &LoadedPresentation,
    summary: &mut ChangeSummary,
) {
    let project_key = |project_id: &str| ProjectKey {
        machine: machine.clone(),
        project_id: project_id.to_string(),
    };
    for project in &next.projects {
        if previous.project(&project.project_id) != Some(project) {
            summary.note_project_changed(project_key(&project.project_id));
        }
    }
    for project in &previous.projects {
        if next.project(&project.project_id).is_none() {
            summary.note_project_removed(project_key(&project.project_id));
        }
    }
    if project_id_order(&previous.projects) != project_id_order(&next.projects) {
        summary.note_project_order_changed(machine.clone());
    }
    for (project_id, session_id, session) in next.sessions.iter() {
        let before = previous.sessions.get(project_id, session_id);
        if before != Some(session) {
            summary.note_session_changed(SessionKey {
                machine: machine.clone(),
                project_id: project_id.to_string(),
                session_id: session_id.to_string(),
            });
        }
        // A row can enter or leave the tab lists without moving in its group.
        if before.is_some_and(|before| tab_listing(before) != tab_listing(session)) {
            summary.note_session_order_changed(project_key(project_id));
        }
    }
    for (project_id, session_id, _) in previous.sessions.iter() {
        if !next.sessions.contains(project_id, session_id) {
            summary.note_session_removed(SessionKey {
                machine: machine.clone(),
                project_id: project_id.to_string(),
                session_id: session_id.to_string(),
            });
        }
    }
    for group in &next.groups {
        let before = previous
            .groups
            .iter()
            .find(|candidate| candidate.group_id == group.group_id)
            .map(|candidate| &candidate.session_ids);
        if before != Some(&group.session_ids) {
            summary.note_session_order_changed(project_key(&group.project_id));
        }
    }
    for group in &previous.groups {
        if !next
            .groups
            .iter()
            .any(|candidate| candidate.group_id == group.group_id)
        {
            summary.note_session_order_changed(project_key(&group.project_id));
        }
    }
}

/// After a snapshot: drop overlays the daemon has caught up with, and report sessions whose
/// effective value changed because of that.
pub(super) fn settle_overlays_after_snapshot(
    machine: &MachineId,
    entry: &mut MachinePresentation,
    summary: &mut ChangeSummary,
) {
    let PresentationState::Loaded(loaded) = &entry.state else {
        return;
    };
    let session_key = |project_id: &str, session_id: &str| SessionKey {
        machine: machine.clone(),
        project_id: project_id.to_string(),
        session_id: session_id.to_string(),
    };

    let finished_patches: Vec<(String, String)> = entry
        .overlays
        .session_patches
        .iter()
        .filter(|(project_id, session_id, patch)| {
            // A patch is finished when its session is gone or the daemon row agrees with it.
            match loaded.server_session(project_id, session_id) {
                Some(server) => patch.verdict(server) != PatchVerdict::Pending,
                None => true,
            }
        })
        .map(|(project_id, session_id, _)| (project_id.to_string(), session_id.to_string()))
        .collect();
    for (project_id, session_id) in finished_patches {
        entry
            .overlays
            .session_patches
            .remove(&project_id, &session_id);
    }

    // A hide is finished when the daemon no longer lists the session, or lists it as stopped.
    let finished_hides: Vec<(String, String, bool)> = entry
        .overlays
        .hidden_sessions
        .iter()
        .filter_map(|(project_id, session_id, _)| {
            match loaded.server_session(project_id, session_id) {
                None => Some((project_id.to_string(), session_id.to_string(), false)),
                Some(server) if server.lifecycle_state == LifecycleState::Stopped => {
                    Some((project_id.to_string(), session_id.to_string(), true))
                }
                Some(_) => None,
            }
        })
        .collect();
    for (project_id, session_id, still_listed) in finished_hides {
        entry
            .overlays
            .hidden_sessions
            .remove(&project_id, &session_id);
        if still_listed {
            // The row is shown again, so it is news, and it can be back in a tab list.
            summary.note_session_changed(session_key(&project_id, &session_id));
            summary.note_session_order_changed(ProjectKey {
                machine: machine.clone(),
                project_id,
            });
        }
    }

    let finished_projects: Vec<String> = entry
        .overlays
        .hidden_projects
        .iter()
        .filter(|project_id| loaded.project(project_id).is_none())
        .cloned()
        .collect();
    for project_id in finished_projects {
        entry.overlays.hidden_projects.remove(&project_id);
    }

    // Rows that are still hidden were never shown, so they are not news.
    let overlays = &entry.overlays;
    summary
        .sessions_changed
        .retain(|key| !overlays.is_session_hidden(&key.project_id, &key.session_id));
    summary
        .sessions_removed
        .retain(|key| !overlays.is_session_hidden(&key.project_id, &key.session_id));
}
