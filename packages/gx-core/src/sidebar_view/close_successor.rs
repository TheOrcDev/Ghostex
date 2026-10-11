//! The session Close Project focuses before it parks the project.
//!
//! CDXC:Projects 2026-09-21 WHY:
//! `closeWorkspaceProjectForGroup` used to be posted bare by the store and given its
//! `successorSessionId` by the sidebar page on its way past (the deleted sidebar page's
//! `post`), which resolved it with `resolveCloseProjectSuccessorSessionId` against the list the
//! page drew. With the page going the store computes it here, so a direct route to the runtime
//! carries the same field and the close does not leave the user with no focused session.
//!
//! The rule is read from the TypeScript rather than restated: the successor is looked for in the
//! groups the list DRAWS, in their drawn order (a collection contributes its groups whether it is
//! collapsed or not, which is where this differs from `renderedNativeSidebarSessionIds`), starting
//! after the closing project and then before it nearest first, and only the first AWAKE row of a
//! candidate counts. With none, nothing is named and the host keeps its ordinary behaviour.
//!
//! SEE-ALSO: the deleted React sidebar's `close-project-successor.ts` and the deleted sidebar
//! page's `controller.ts` (`post`),
//! apps/desktop/src/app/gx_store/sidebar_close_project.rs.

use crate::focus::ActiveGroup;

use super::view::{OrderKind, SessionRow, SessionView, SidebarView};

/// `sidebarStore.getState().groupsById[groupId]?.isActive` for a PROJECT group.
///
/// The old runtime marks the project's own group active even for a focused row that sits in one of
/// that project's user-made groups, where the store's active group is the user-made group itself
/// (declared difference 54). Close Project is only ever offered on a project header, so the
/// comparison is made against the project the active group belongs to.
pub fn close_project_group_is_active(active: Option<&ActiveGroup>, group_id: &str) -> bool {
    let Some(active) = active else {
        return false;
    };
    let active_project_group_id = match active {
        ActiveGroup::Subgroup { project, .. } => ActiveGroup::Project(project.clone()),
        other => other.clone(),
    }
    .to_sidebar_group_id();
    active_project_group_id == group_id
}

/// `orderedGroupIds`: every drawn top-level item flattened into project group ids.
///
/// A collection contributes its `groupIds` even when it is collapsed, because the TypeScript's
/// lookup here has no `!collection.collapsed` filter; the rendered-rows helper beside it does, and
/// copying that one would silently skip a folder the user closed.
pub fn close_project_successor_group_order(view: &SidebarView) -> Vec<String> {
    let mut ordered = Vec::with_capacity(view.order.len());
    for item in &view.order {
        match item.kind {
            OrderKind::Project => ordered.push(item.id.clone()),
            OrderKind::Collection => {
                if let Some(collection) = view
                    .collections
                    .iter()
                    .find(|collection| collection.collection_id == item.id)
                {
                    ordered.extend(collection.group_ids.iter().cloned());
                }
            }
        }
    }
    ordered
}

/// The groups the successor is looked for in, in the order they are tried: the ones after the
/// closing project, then the ones before it nearest first. Empty when the closing project is not
/// in the drawn order, which is the TypeScript's `closingIndex === -1` early return.
pub fn close_project_successor_candidates(
    view: &SidebarView,
    closing_group_id: &str,
) -> Vec<String> {
    let ordered = close_project_successor_group_order(view);
    let Some(closing) = ordered.iter().position(|id| id == closing_group_id) else {
        return Vec::new();
    };
    let mut candidates: Vec<String> = ordered[closing + 1..].to_vec();
    candidates.extend(ordered[..closing].iter().rev().cloned());
    candidates
}

/// `isAwakeWorkspaceSession`: a row that focusing does not wake, and not a browser tab.
///
/// The TypeScript tests `isSleeping !== true` and `lifecycleState !== 'sleeping'` separately; the
/// store's row carries one value for both (`rows.rs` writes `isSleeping` from it), so one test here
/// is the same answer. `kind` and `sessionKind` are likewise one flag on the row.
pub fn is_awake_successor_row(row: &SessionRow) -> bool {
    !row.is_browser
        && row.session_kind.as_deref() != Some("browser")
        && row.lifecycle_state != "sleeping"
}

/// The first row of a candidate group that may be focused, in the order the group holds its rows.
pub fn first_awake_successor_session_id(rows: &[SessionView]) -> Option<&str> {
    rows.iter()
        .find(|session| is_awake_successor_row(&session.row))
        .map(|session| session.row.sidebar_session_id.as_str())
}
