//! Reducers for one delta: whole-object upserts and removals on a loaded machine.

use ghostex_gx_protocol::{
    LifecycleState, PresentationGroup, PresentationProject, PresentationSession,
};

use super::loaded::{
    project_id_order, sort_groups, sort_projects, tab_listing, LoadedPresentation,
};
use crate::change::ChangeSummary;
use crate::keys::{MachineId, ProjectKey, SessionKey};
use crate::overlay::{Overlays, PatchVerdict};

pub(super) fn upsert_session(
    machine: &MachineId,
    loaded: &mut LoadedPresentation,
    overlays: &mut Overlays,
    session: PresentationSession,
    summary: &mut ChangeSummary,
) {
    let project_id = session.project_id.clone();
    let session_id = session.session_id.clone();
    let group_id = session.group_id.clone();
    let key = SessionKey {
        machine: machine.clone(),
        project_id: project_id.clone(),
        session_id: session_id.clone(),
    };

    // A patch ends when the daemon row says the same thing, or says something newer than the
    // value the patch was made against.
    let patch_done = overlays
        .session_patches
        .get(&project_id, &session_id)
        .is_some_and(|stored| stored.verdict(&session) != PatchVerdict::Pending);
    if patch_done {
        overlays.session_patches.remove(&project_id, &session_id);
    }
    // A locally closed session that the daemon now reports as stopped is no longer a live row the
    // hide has to keep out; what remains is the daemon's own stopped row (pinned, favorite, or
    // tagged), which the user must still be able to see.
    let unhidden = session.lifecycle_state == LifecycleState::Stopped
        && overlays
            .hidden_sessions
            .remove(&project_id, &session_id)
            .is_some();

    let previous = loaded.sessions.insert(&project_id, &session_id, session);
    let changed = match (&previous, loaded.sessions.get(&project_id, &session_id)) {
        (Some(previous), Some(current)) => previous != current,
        _ => true,
    };
    let hidden = overlays.is_session_hidden(&project_id, &session_id);
    if (changed || patch_done || unhidden) && !hidden {
        summary.note_session_changed(key);
    }

    // A row can enter or leave the tab lists without moving in its group: a session that stops
    // keeps its place in `sessionIds` but is no longer listed in the sidebar.
    let listing_changed = match (&previous, loaded.sessions.get(&project_id, &session_id)) {
        (Some(previous), Some(current)) => tab_listing(previous) != tab_listing(current),
        _ => false,
    };
    let mut order_changed =
        loaded.rebuild_group_session_ids(&project_id, &group_id) || listing_changed;
    if let Some(previous) = previous.filter(|previous| previous.group_id != group_id) {
        order_changed |= loaded.rebuild_group_session_ids(&project_id, &previous.group_id);
    }
    if order_changed || unhidden {
        summary.note_session_order_changed(ProjectKey {
            machine: machine.clone(),
            project_id,
        });
    }
}

pub(super) fn remove_session(
    machine: &MachineId,
    loaded: &mut LoadedPresentation,
    overlays: &mut Overlays,
    project_id: &str,
    session_id: &str,
    summary: &mut ChangeSummary,
) {
    // The daemon agrees the session is gone, so the overlays about it have nothing left to do.
    let was_hidden = overlays.is_session_hidden(project_id, session_id);
    overlays.hidden_sessions.remove(project_id, session_id);
    overlays.session_patches.remove(project_id, session_id);
    let Some(removed) = loaded.sessions.remove(project_id, session_id) else {
        return;
    };
    loaded.rebuild_group_session_ids(project_id, &removed.group_id);
    if !was_hidden {
        summary.note_session_removed(SessionKey {
            machine: machine.clone(),
            project_id: project_id.to_string(),
            session_id: session_id.to_string(),
        });
        summary.note_session_order_changed(ProjectKey {
            machine: machine.clone(),
            project_id: project_id.to_string(),
        });
    }
}

pub(super) fn upsert_project(
    machine: &MachineId,
    loaded: &mut LoadedPresentation,
    project: PresentationProject,
    summary: &mut ChangeSummary,
) {
    let project_id = project.project_id.clone();
    let group_id = project.default_group_id();
    let group_sort_key = format!("{}:active", project.sort_key);
    let order_before = project_id_order(&loaded.projects);

    let changed = match loaded
        .projects
        .iter_mut()
        .find(|existing| existing.project_id == project_id)
    {
        Some(existing) => {
            let changed = *existing != project;
            *existing = project;
            changed
        }
        None => {
            loaded.projects.push(project);
            true
        }
    };
    sort_projects(&mut loaded.projects);

    let mut new_group = false;
    match loaded
        .groups
        .iter_mut()
        .find(|group| group.project_id == project_id || group.group_id == group_id)
    {
        Some(group) => {
            group.group_id = group_id.clone();
            group.project_id = project_id.clone();
            group.sort_key = group_sort_key;
        }
        None => {
            new_group = true;
            loaded.groups.push(PresentationGroup {
                group_id: group_id.clone(),
                project_id: project_id.clone(),
                session_ids: Vec::new(),
                sort_key: group_sort_key,
                title: "Active".to_string(),
            });
        }
    }
    sort_groups(&mut loaded.groups);
    // Sessions can arrive before their project. The TypeScript reducer left a new group empty
    // until some later session delta rebuilt it; filling it here shows those sessions at once.
    let filled = new_group && loaded.rebuild_group_session_ids(&project_id, &group_id);

    let key = ProjectKey {
        machine: machine.clone(),
        project_id,
    };
    if changed {
        summary.note_project_changed(key.clone());
    }
    if filled {
        summary.note_session_order_changed(key);
    }
    if order_before != project_id_order(&loaded.projects) {
        summary.note_project_order_changed(machine.clone());
    }
}

pub(super) fn remove_project(
    machine: &MachineId,
    loaded: &mut LoadedPresentation,
    overlays: &mut Overlays,
    project_id: &str,
    summary: &mut ChangeSummary,
) {
    let was_hidden = overlays.hidden_projects.contains(project_id);
    let hidden_sessions: Vec<String> = overlays
        .hidden_sessions
        .project(project_id)
        .map(|sessions| sessions.keys().cloned().collect())
        .unwrap_or_default();
    overlays.clear_project(project_id);

    let existed = loaded.project(project_id).is_some();
    loaded
        .projects
        .retain(|project| project.project_id != project_id);
    loaded.groups.retain(|group| group.project_id != project_id);
    let removed_sessions = loaded.sessions.remove_project(project_id);
    if was_hidden {
        return;
    }
    for session_id in removed_sessions.into_keys() {
        if !hidden_sessions.contains(&session_id) {
            summary.note_session_removed(SessionKey {
                machine: machine.clone(),
                project_id: project_id.to_string(),
                session_id,
            });
        }
    }
    if existed {
        summary.note_project_removed(ProjectKey {
            machine: machine.clone(),
            project_id: project_id.to_string(),
        });
        summary.note_project_order_changed(machine.clone());
    }
}
