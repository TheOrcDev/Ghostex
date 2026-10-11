//! The four remote sidebar writes that had no allowlist entry, and their param shaping.
//!
//! CDXC:RemoteMachines 2026-09-21 WHY:
//! Editing a Space, reordering projects, Switch Account on a session and saving a note all work on
//! this computer and all failed on a remote machine's tab, because the old runtime posts each one
//! down the machine's tunnel (`updateRemoteSidebarSpaces`, `updateRemoteWorkspaceGroups`, the
//! remote legs of `switchSessionAgent` and `saveSessionNote`) while
//! `gpui_remote_sidebar_request_path_allowed` never listed the four paths, so the call was refused
//! at the Rust boundary and the user saw a failed action with no reason. They were missed because
//! each arrived with a feature whose LOCAL leg was the whole of the work: the allowlist is a
//! separate list that only the remote leg reads, and nothing fails at build time when a path is
//! absent from it.
//!
//! Each is the same kind of write as an allowlisted sibling and is shaped here the same way. The
//! two documents are whole-state replacements like `/api/updateSidebarProjectCollections`, so their
//! params are rebuilt field by field from bounded ids, names, icon ids and colors, and the machine's
//! answer is rebuilt through the same function before the renderer sees it: nothing that arrives
//! from a remote daemon reaches CEF unshaped. The two session writes are id-scoped like
//! `/api/settleSession`, plus one bounded extra field each (the agent id, which is already the
//! rename route's `agentName` rule, and the note body, which is user prose and so is bounded by
//! length and NUL only, the same allowance `/api/sendSessionMessage` makes for a prompt).
//!
//! The server needs no change. `/api/switchSessionAgent`, `/api/updateWorkspaceSessionGroups` and
//! `/api/updateSidebarSpaces` are already `RemoteAllowed` in `server/src/protocol.rs`, and
//! `/api/saveSessionAgentNote` is deliberately `FullLocal` there (`CDXC:SessionNotes 2026-08-24`:
//! a note is user-authored prose) which this route does not weaken: the saved-machine tunnel
//! reaches that daemon's own authenticated LOCAL listener, exactly as
//! `/api/generateCommitMessage` and `/api/createPullRequest` do.
//!
//! SEE-ALSO: apps/desktop/src/app/helpers/remote/sidebar_bridge/ (the allowlist, the shaping
//! dispatch and the response dispatch), apps/desktop/src/app/remote_conn/sidebar_rpc.rs (the one
//! function both callers go through).

use crate::app::helpers::*;

/// The longest a note may be on the way to the daemon, which enforces the same limit itself
/// (`SESSION_AGENT_NOTE_MAX_CHARS`). An empty note is a deletion and is deliberately allowed.
const GPUI_REMOTE_SIDEBAR_NOTE_MAX_CHARS: usize = 4_096;

const GPUI_REMOTE_SIDEBAR_MAX_SPACES: usize = 256;
const GPUI_REMOTE_SIDEBAR_MAX_SPACE_MEMBERS: usize = 512;
const GPUI_REMOTE_SIDEBAR_MAX_PROJECT_GROUPS: usize = 512;
const GPUI_REMOTE_SIDEBAR_MAX_GROUPS_PER_PROJECT: usize = 256;
const GPUI_REMOTE_SIDEBAR_MAX_SESSIONS_PER_GROUP: usize = 512;
const GPUI_REMOTE_SIDEBAR_MAX_ID_CHARS: usize = 256;
const GPUI_REMOTE_SIDEBAR_MAX_TITLE_CHARS: usize = 256;

/// One bounded single-line label: a Space name, a group title, an icon id.
fn bounded_label(candidate: &str, max_chars: usize) -> Option<&str> {
    let trimmed = candidate.trim();
    (!trimmed.is_empty()
        && trimmed.chars().count() <= max_chars
        && !trimmed.contains('\0')
        && !trimmed.chars().any(char::is_control))
    .then_some(trimmed)
}

/// The same colour rule the collections document uses.
fn valid_color(candidate: &str) -> bool {
    candidate == "transparent"
        || (candidate.len() == 7
            && candidate.starts_with('#')
            && candidate[1..].bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// `{ state }` for `/api/updateSidebarSpaces`.
pub(crate) fn gpui_remote_sidebar_spaces_params(
    params: serde_json::Value,
) -> Option<serde_json::Value> {
    let state = gpui_remote_sidebar_spaces_state(params.get("state")?)?;
    Some(serde_json::json!({ "state": state }))
}

/// The Spaces document, rebuilt field by field.
///
/// A Space is an id, a name, an icon id, a colour and two membership lists. Every `order` entry
/// must name a Space the document holds, once, so an order entry with no Space behind it is refused
/// here rather than sent; the count is deliberately NOT required to match, which is the one place
/// this differs from the collections document, because the page builds the map by walking the order
/// and a Space the order has lost is still the machine's to keep.
pub(crate) fn gpui_remote_sidebar_spaces_state(
    value: &serde_json::Value,
) -> Option<serde_json::Value> {
    let source = value.as_object()?;
    let source_spaces = source.get("spaces")?.as_object()?;
    let source_order = source.get("order")?.as_array()?;
    if source_spaces.len() > GPUI_REMOTE_SIDEBAR_MAX_SPACES
        || source_order.len() > GPUI_REMOTE_SIDEBAR_MAX_SPACES
    {
        return None;
    }
    let mut spaces = serde_json::Map::new();
    for (space_id, candidate) in source_spaces {
        let normalized_space_id = bounded_label(space_id, GPUI_REMOTE_SIDEBAR_MAX_ID_CHARS)?;
        let candidate = candidate.as_object()?;
        if candidate.get("spaceId")?.as_str()? != normalized_space_id {
            return None;
        }
        let name = bounded_label(
            candidate.get("name")?.as_str()?,
            GPUI_REMOTE_SIDEBAR_MAX_TITLE_CHARS,
        )?;
        let icon = bounded_label(
            candidate.get("icon")?.as_str()?,
            GPUI_REMOTE_SIDEBAR_MAX_ID_CHARS,
        )?;
        let color = candidate.get("color")?.as_str()?;
        if !valid_color(color) {
            return None;
        }
        let member_project_ids = gpui_remote_sidebar_space_project_ids(candidate)?;
        let member_collection_ids = gpui_remote_sidebar_space_collection_ids(candidate)?;
        spaces.insert(
            normalized_space_id.to_string(),
            serde_json::json!({
                "color": color,
                "icon": icon,
                "memberCollectionIds": member_collection_ids,
                "memberProjectIds": member_project_ids,
                "name": name,
                "spaceId": normalized_space_id,
            }),
        );
    }
    let mut order = Vec::with_capacity(source_order.len());
    let mut seen = std::collections::HashSet::new();
    for space_id in source_order {
        let space_id = bounded_label(space_id.as_str()?, GPUI_REMOTE_SIDEBAR_MAX_ID_CHARS)?;
        if !spaces.contains_key(space_id) || !seen.insert(space_id.to_string()) {
            return None;
        }
        order.push(serde_json::Value::String(space_id.to_string()));
    }
    Some(serde_json::json!({ "order": order, "spaces": spaces }))
}

fn gpui_remote_sidebar_space_project_ids(
    space: &serde_json::Map<String, serde_json::Value>,
) -> Option<Vec<serde_json::Value>> {
    let source = space.get("memberProjectIds")?.as_array()?;
    if source.len() > GPUI_REMOTE_SIDEBAR_MAX_SPACE_MEMBERS {
        return None;
    }
    let mut project_ids = Vec::with_capacity(source.len());
    for project_id in source {
        let project_id = project_id.as_str()?.trim();
        if !gpui_remote_sidebar_project_id_allowed(project_id) {
            return None;
        }
        project_ids.push(serde_json::Value::String(project_id.to_string()));
    }
    Some(project_ids)
}

fn gpui_remote_sidebar_space_collection_ids(
    space: &serde_json::Map<String, serde_json::Value>,
) -> Option<Vec<serde_json::Value>> {
    let source = space.get("memberCollectionIds")?.as_array()?;
    if source.len() > GPUI_REMOTE_SIDEBAR_MAX_SPACE_MEMBERS {
        return None;
    }
    let mut collection_ids = Vec::with_capacity(source.len());
    for collection_id in source {
        let collection_id =
            bounded_label(collection_id.as_str()?, GPUI_REMOTE_SIDEBAR_MAX_ID_CHARS)?;
        collection_ids.push(serde_json::Value::String(collection_id.to_string()));
    }
    Some(collection_ids)
}

/// `{ state }` for `/api/updateWorkspaceSessionGroups`.
pub(crate) fn gpui_remote_sidebar_workspace_groups_params(
    params: serde_json::Value,
) -> Option<serde_json::Value> {
    let state = gpui_remote_sidebar_workspace_groups_state(params.get("state")?)?;
    Some(serde_json::json!({ "state": state }))
}

/// The workspace session groups document, rebuilt field by field.
///
/// `projectOrder` is the project reorder itself; `projects` carries each project's user-made groups
/// and their members. Unlike the two other documents the order may name a project the map does not
/// hold and the other way round: the daemon keeps groups only for the projects that have one, while
/// the order lists every project in the sidebar.
pub(crate) fn gpui_remote_sidebar_workspace_groups_state(
    value: &serde_json::Value,
) -> Option<serde_json::Value> {
    let source = value.as_object()?;
    let source_order = source.get("projectOrder")?.as_array()?;
    let source_projects = source.get("projects")?.as_object()?;
    if source_order.len() > GPUI_REMOTE_SIDEBAR_MAX_PROJECT_GROUPS
        || source_projects.len() > GPUI_REMOTE_SIDEBAR_MAX_PROJECT_GROUPS
    {
        return None;
    }
    let mut project_order = Vec::with_capacity(source_order.len());
    let mut seen = std::collections::HashSet::new();
    for project_id in source_order {
        let project_id = project_id.as_str()?.trim();
        if !gpui_remote_sidebar_project_id_allowed(project_id)
            || !seen.insert(project_id.to_string())
        {
            return None;
        }
        project_order.push(serde_json::Value::String(project_id.to_string()));
    }
    let mut projects = serde_json::Map::new();
    for (project_id, candidate) in source_projects {
        if !gpui_remote_sidebar_project_id_allowed(project_id.trim()) {
            return None;
        }
        let candidate = candidate.as_object()?;
        let source_groups = candidate.get("groups")?.as_array()?;
        if source_groups.len() > GPUI_REMOTE_SIDEBAR_MAX_GROUPS_PER_PROJECT {
            return None;
        }
        let mut groups = Vec::with_capacity(source_groups.len());
        for group in source_groups {
            groups.push(gpui_remote_sidebar_workspace_group(group)?);
        }
        let mut project = serde_json::Map::new();
        project.insert("groups".to_string(), serde_json::Value::Array(groups));
        // Optional in the document, so it rides through only when the machine's own copy had one.
        if let Some(next_group_number) = candidate.get("nextGroupNumber") {
            let next_group_number = next_group_number.as_u64().filter(|value| *value >= 1)?;
            project.insert(
                "nextGroupNumber".to_string(),
                serde_json::json!(next_group_number),
            );
        }
        projects.insert(
            project_id.trim().to_string(),
            serde_json::Value::Object(project),
        );
    }
    Some(serde_json::json!({ "projectOrder": project_order, "projects": projects }))
}

fn gpui_remote_sidebar_workspace_group(group: &serde_json::Value) -> Option<serde_json::Value> {
    let group = group.as_object()?;
    let group_id = bounded_label(
        group.get("groupId")?.as_str()?,
        GPUI_REMOTE_SIDEBAR_MAX_ID_CHARS,
    )?;
    // A group's title may be empty: a group created from a session starts unnamed.
    let title = group.get("title")?.as_str()?.trim();
    if title.chars().count() > GPUI_REMOTE_SIDEBAR_MAX_TITLE_CHARS
        || title.contains('\0')
        || title.chars().any(char::is_control)
    {
        return None;
    }
    let source_session_ids = group.get("sessionIds")?.as_array()?;
    if source_session_ids.len() > GPUI_REMOTE_SIDEBAR_MAX_SESSIONS_PER_GROUP {
        return None;
    }
    let mut session_ids = Vec::with_capacity(source_session_ids.len());
    for session_id in source_session_ids {
        let session_id = session_id.as_str()?.trim();
        if !gpui_remote_sidebar_session_id_allowed(session_id) {
            return None;
        }
        session_ids.push(serde_json::Value::String(session_id.to_string()));
    }
    Some(serde_json::json!({
        "groupId": group_id,
        "sessionIds": session_ids,
        "title": title,
    }))
}

/// The two ids plus the agent id, for `/api/switchSessionAgent`.
pub(crate) fn gpui_remote_sidebar_switch_session_agent_params(
    params: serde_json::Value,
) -> Option<serde_json::Value> {
    let agent_id = params
        .get("agentId")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| gpui_remote_sidebar_agent_id_allowed(value))?
        .to_string();
    let mut shaped = gpui_remote_sidebar_session_lifecycle_params(params, None)?;
    shaped["agentId"] = serde_json::Value::String(agent_id);
    Some(shaped)
}

/// The two ids plus the note body, for `/api/saveSessionAgentNote`.
///
/// The note is prose the user typed, so newlines and tabs are legal where every other bridged
/// string forbids control characters, and an EMPTY note is a real value: it is how a note is
/// deleted.
pub(crate) fn gpui_remote_sidebar_session_note_params(
    params: serde_json::Value,
) -> Option<serde_json::Value> {
    let note = params
        .get("note")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            value.chars().count() <= GPUI_REMOTE_SIDEBAR_NOTE_MAX_CHARS
                && !value.contains('\0')
                && !value
                    .chars()
                    .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
        })?
        .to_string();
    let mut shaped = gpui_remote_sidebar_session_lifecycle_params(params, None)?;
    shaped["note"] = serde_json::Value::String(note);
    Some(shaped)
}

/// The longest account id a pick may name. The daemon's ids are short slugs.
const GPUI_REMOTE_SIDEBAR_MAX_ACCOUNT_ID_CHARS: usize = 128;

/// `/api/agentAccounts` for the sidebar's account pages, and only its three read-or-pick
/// operations: the launcher's `list`, a row's `session`, and a row's `select`.
///
/// CDXC:RemoteMachines 2026-09-21 WHY:
/// The account pages are the store's now (gx_store/sidebar_accounts.rs) and their remote calls go
/// through the one shared function, whose allowlist never carried this path: the old runtime sent
/// both pages down a machine's tunnel and every one of them failed at this boundary, so a remote
/// project's launcher never showed its accounts and a remote row's Switch Account always showed
/// the failure. The endpoint also sets up, edits and removes accounts, which a sidebar menu never
/// does, so every other operation is refused here, as is any field beyond the ones these three
/// read. `/api/agentAccounts` is already `RemoteAllowed` in `server/src/protocol.rs`.
pub(crate) fn gpui_remote_sidebar_agent_accounts_params(
    params: serde_json::Value,
) -> Option<serde_json::Value> {
    let operation = params.get("operation")?.as_str()?;
    let refresh = match params.get("refresh") {
        None => None,
        Some(serde_json::Value::Bool(refresh)) => Some(*refresh),
        Some(_) => return None,
    };
    let mut shaped = match operation {
        "list" => serde_json::json!({}),
        "session" => gpui_remote_sidebar_session_lifecycle_params(params.clone(), None)?,
        "select" => {
            let account_id = params
                .get("accountId")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| {
                    bounded_label(value, GPUI_REMOTE_SIDEBAR_MAX_ACCOUNT_ID_CHARS)
                        .filter(|trimmed| *trimmed == value && !value.contains(['/', '\\']))
                })?
                .to_string();
            let mut shaped = gpui_remote_sidebar_session_lifecycle_params(params.clone(), None)?;
            shaped["accountId"] = serde_json::Value::String(account_id);
            // A pick never refreshes.
            if refresh.is_some() {
                return None;
            }
            shaped
        }
        _ => return None,
    };
    shaped["operation"] = serde_json::Value::String(operation.to_string());
    if let Some(refresh) = refresh {
        shaped["refresh"] = serde_json::Value::Bool(refresh);
    }
    Some(shaped)
}

/// The machine's account list cut to what the account pages read (gx-core's `AccountsState`),
/// so nothing else of the answer reaches the old runtime or the store. An answer that is not an
/// account list becomes `null`, which the pages read as a failed call.
pub(crate) fn gpui_remote_sidebar_agent_accounts_response_payload(
    result: serde_json::Value,
) -> serde_json::Value {
    ghostex_gx_core::AccountsState::from_json(&result)
        .map(|state| state.to_json())
        .unwrap_or(serde_json::Value::Null)
}
