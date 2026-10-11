use super::*;
use crate::*;

impl WorkspaceModel {
    pub(crate) fn rotate_panes_clockwise(&mut self) -> bool {
        /*
        CDXC:CommandPane 2026-06-26-06:57:
        Native Agents Rotate Panes Clockwise is a pure split-tree transform: recursively swap horizontal and vertical axes, reverse vertical branches while inverting their ratios, and preserve leaf pane ids, tab order, active tabs, focused pane, and terminal presentation records. Single-leaf workspaces no-op, and command-pane state stays outside this model.

        CDXC:CommandPane 2026-06-26-06:57:
        Existing Agents geometry mutations clear Focus mode when the visible split layout changes. Rotation follows that rule after a multi-leaf transform so the rotated pane tree is immediately visible while the selected focused pane id remains stable for follow-up actions.
        */
        if self.leaf_order().len() <= 1 {
            return false;
        }

        rotate_workspace_node_clockwise(&mut self.root);
        self.focus_mode_pane = None;
        true
    }

    pub(crate) fn split_mounting_session_adjacent_to_pane(
        &mut self,
        requested_pane_id: WorkspacePaneId,
        axis: WorkspaceSplitAxis,
        new_leaf_first: bool,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        /*
        CDXC:Terminal 2026-06-22-23:33:
        Agents explicit split controls must offer parity for Split Right and Split Below from the pane tab chrome. Each control creates a new split leaf with a selected Mounting terminal, uses the existing split tree and persistence path, and clears Focus mode only so the newly created pane is visible without adding fake Running state, overlays, native hit-test routing, libghostty mounts, or real process creation.
        */
        let target_pane_id = self.resolve_action_pane_id(requested_pane_id)?;
        let session_id = self.allocate_session_id();
        let pane_id = self.allocate_pane_id();
        let split_id = self.allocate_split_id();
        let new_leaf = WorkspaceLeaf {
            pane_id,
            tab_group: WorkspaceTabGroup {
                tabs: vec![WorkspaceTab { session_id }],
                active_tab: session_id,
            },
        };

        if insert_workspace_leaf_split(
            &mut self.root,
            target_pane_id,
            new_leaf,
            axis,
            new_leaf_first,
            split_id,
        ) {
            self.terminal_sessions.push(TerminalSession::placeholder(
                session_id,
                terminal_session_title_for_id(session_id),
                TerminalSessionPresentationState::Mounting,
            ));
            self.set_focused_pane(pane_id);
            self.focus_mode_pane = None;
            self.normalize_workspace_tree();
            Some((pane_id, session_id))
        } else {
            None
        }
    }

    pub(crate) fn add_placeholder_session_from_command_title(
        &mut self,
        target_pane_id: WorkspacePaneId,
        title: String,
        zone: WorkspaceDropZone,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        /*
        CDXC:Workarea 2026-06-22-23:33:
        Command-pane tabs dropped onto an Agents pane body become selected Mounting Agents shell sessions with the command tab's visible title. Center drops group into the target pane, edge drops use normal Agents split semantics, and this remains shell state only: no command process, terminal content, stdout/stderr, libghostty mount/remount, fake Running state, overlay, or hidden hit region is transferred.
        */
        match zone {
            WorkspaceDropZone::Center => {
                self.group_placeholder_session_from_command_title(target_pane_id, title)
            }
            WorkspaceDropZone::Left
            | WorkspaceDropZone::Right
            | WorkspaceDropZone::Top
            | WorkspaceDropZone::Bottom => {
                self.split_placeholder_session_from_command_title(target_pane_id, title, zone)
            }
        }
    }

    pub(crate) fn group_placeholder_session_from_command_title(
        &mut self,
        target_pane_id: WorkspacePaneId,
        title: String,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        let insertion_index = self.find_leaf(target_pane_id)?.tab_group.tabs.len();
        self.insert_placeholder_session_from_command_title_at(
            target_pane_id,
            insertion_index,
            title,
        )
    }

    pub(crate) fn insert_placeholder_session_from_command_title_at(
        &mut self,
        target_pane_id: WorkspacePaneId,
        insertion_index: usize,
        title: String,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        /*
        CDXC:Workarea 2026-06-22-23:33:
        Command tabs dropped on an Agents tab strip insert a new Mounting Agents shell session at the visible tab boundary or end target, select it, and focus that Agents pane. This is still a placeholder boundary: only the visible command title crosses surfaces, with no process, command text, stdout/stderr, terminal content, libghostty mount/remount, fake Running state, real Source/Kanban/Automate/Manage surface, overlay, hidden hit region, or native/root hit-test routing.
        */
        self.find_leaf(target_pane_id)?;
        let session_id = self.allocate_session_id();
        self.terminal_sessions.push(TerminalSession::placeholder(
            session_id,
            title,
            TerminalSessionPresentationState::Mounting,
        ));
        let tab = WorkspaceTab { session_id };

        let Some(target_leaf) = self.find_leaf_mut(target_pane_id) else {
            self.terminal_sessions
                .retain(|session| session.id != session_id);
            return None;
        };
        target_leaf
            .tab_group
            .insert_session_at(tab, insertion_index);
        target_leaf.tab_group.active_tab = session_id;
        self.set_focused_pane(target_pane_id);
        Some((target_pane_id, session_id))
    }

    pub(crate) fn split_placeholder_session_from_command_title(
        &mut self,
        target_pane_id: WorkspacePaneId,
        title: String,
        zone: WorkspaceDropZone,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        if matches!(zone, WorkspaceDropZone::Center) {
            return self.group_placeholder_session_from_command_title(target_pane_id, title);
        }
        self.find_leaf(target_pane_id)?;

        let session_id = self.allocate_session_id();
        let pane_id = self.allocate_pane_id();
        let split_id = self.allocate_split_id();
        let new_leaf = WorkspaceLeaf {
            pane_id,
            tab_group: WorkspaceTabGroup {
                tabs: vec![WorkspaceTab { session_id }],
                active_tab: session_id,
            },
        };
        let axis = match zone {
            WorkspaceDropZone::Left | WorkspaceDropZone::Right => WorkspaceSplitAxis::Horizontal,
            WorkspaceDropZone::Top | WorkspaceDropZone::Bottom => WorkspaceSplitAxis::Vertical,
            WorkspaceDropZone::Center => unreachable!("center grouping handled above"),
        };
        let dragged_first = matches!(zone, WorkspaceDropZone::Left | WorkspaceDropZone::Top);

        if insert_workspace_leaf_split(
            &mut self.root,
            target_pane_id,
            new_leaf,
            axis,
            dragged_first,
            split_id,
        ) {
            self.terminal_sessions.push(TerminalSession::placeholder(
                session_id,
                title,
                TerminalSessionPresentationState::Mounting,
            ));
            self.set_focused_pane(pane_id);
            self.focus_mode_pane = None;
            self.normalize_workspace_tree();
            Some((pane_id, session_id))
        } else {
            None
        }
    }

    pub(crate) fn group_tab_into_pane(
        &mut self,
        source_pane_id: WorkspacePaneId,
        target_pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
    ) -> bool {
        if !self.has_session(session_id) || self.find_leaf(target_pane_id).is_none() {
            return false;
        }

        if source_pane_id == target_pane_id {
            self.select_tab(target_pane_id, session_id);
            return true;
        }

        let Some((tab, source_is_empty)) = self.remove_tab_for_move(source_pane_id, session_id)
        else {
            return false;
        };

        if source_is_empty {
            self.collapse_empty_leaf(source_pane_id);
        }
        self.clear_focus_mode_if_invalid();

        let Some(target_leaf) = self.find_leaf_mut(target_pane_id) else {
            return false;
        };
        target_leaf
            .tab_group
            .insert_session_at(tab, target_leaf.tab_group.tabs.len());
        target_leaf.tab_group.active_tab = session_id;
        self.set_focused_pane(target_pane_id);
        self.normalize_workspace_tree();
        true
    }

    pub(crate) fn split_tab_to_pane(
        &mut self,
        source_pane_id: WorkspacePaneId,
        target_pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
        zone: WorkspaceDropZone,
    ) -> bool {
        /*
        CDXC:Workarea 2026-06-22-05:31:
        Pane-body Agents tab drops use an in-memory layout mutation only in this slice: center drops group into the target tab group, while left/right/top/bottom edge drops create a new leaf beside the target and remove the dragged tab from its source. Empty source leaves are collapsed immediately so the split tree remains renderable without persistence, command-pane drag/drop, browser CEF drag behavior, or real wake/mount work.
        */
        if matches!(zone, WorkspaceDropZone::Center) {
            return self.group_tab_into_pane(source_pane_id, target_pane_id, session_id);
        }

        if !self.has_session(session_id) || self.find_leaf(target_pane_id).is_none() {
            return false;
        }

        if self.workspace_tab_edge_drop_is_single_tab_own_pane_noop(
            source_pane_id,
            target_pane_id,
            zone,
        ) {
            return false;
        }

        let Some((tab, source_is_empty)) = self.remove_tab_for_move(source_pane_id, session_id)
        else {
            return false;
        };

        if source_is_empty {
            self.collapse_empty_leaf(source_pane_id);
        }
        self.clear_focus_mode_if_invalid();

        let pane_id = self.allocate_pane_id();
        let split_id = self.allocate_split_id();
        let new_leaf = WorkspaceLeaf {
            pane_id,
            tab_group: WorkspaceTabGroup {
                tabs: vec![tab],
                active_tab: session_id,
            },
        };
        let axis = match zone {
            WorkspaceDropZone::Left | WorkspaceDropZone::Right => WorkspaceSplitAxis::Horizontal,
            WorkspaceDropZone::Top | WorkspaceDropZone::Bottom => WorkspaceSplitAxis::Vertical,
            WorkspaceDropZone::Center => unreachable!("center grouping handled above"),
        };
        let dragged_first = matches!(zone, WorkspaceDropZone::Left | WorkspaceDropZone::Top);

        if insert_workspace_leaf_split(
            &mut self.root,
            target_pane_id,
            new_leaf,
            axis,
            dragged_first,
            split_id,
        ) {
            self.set_focused_pane(pane_id);
            self.normalize_workspace_tree();
            true
        } else {
            false
        }
    }

    pub(crate) fn remove_tab_for_move(
        &mut self,
        pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
    ) -> Option<(WorkspaceTab, bool)> {
        let leaf = self.find_leaf_mut(pane_id)?;
        let tab = leaf.tab_group.remove_session(session_id)?;
        let source_is_empty = leaf.tab_group.tabs.is_empty();
        Some((tab, source_is_empty))
    }

    pub(crate) fn collapse_empty_leaf(&mut self, pane_id: WorkspacePaneId) {
        /*
        CDXC:CommandPane 2026-06-22-10:23:
        Closing the only Agents tab in a split pane must choose the next keyboard target from the pre-collapse sibling branch, while same-pane closes keep the right-then-left tab selection owned by WorkspaceTabGroup::remove_session. Sibling-branch candidates are scored before the broader pane geometry fallback so nested layouts match native close-focus behavior.
        */
        let replacement_focus = workspace_close_focus_replacement_leaf_id(&self.root, pane_id);
        let root_is_empty = collapse_empty_workspace_leaf(&mut self.root, pane_id);
        if root_is_empty {
            self.root = workspace_empty_leaf_node(pane_id);
            self.set_focused_pane(pane_id);
        }

        if self.focused_pane == pane_id || self.find_leaf(self.focused_pane).is_none() {
            let next_focus = replacement_focus
                .filter(|pane_id| self.find_leaf(*pane_id).is_some())
                .or_else(|| self.most_recent_pane_where(|_| true))
                .or_else(|| first_workspace_leaf_id(&self.root));
            if let Some(next_focus) = next_focus {
                self.set_focused_pane(next_focus);
            }
        }
        self.prune_pane_focus_history();
    }

    pub(crate) fn allocate_pane_id(&mut self) -> WorkspacePaneId {
        let pane_id = WorkspacePaneId(self.next_pane_id);
        self.next_pane_id += 1;
        pane_id
    }

    pub(crate) fn allocate_split_id(&mut self) -> WorkspaceSplitId {
        let split_id = WorkspaceSplitId(self.next_split_id);
        self.next_split_id += 1;
        split_id
    }

    pub(crate) fn allocate_session_id(&mut self) -> TerminalSessionId {
        let session_id = TerminalSessionId(self.next_session_id);
        self.next_session_id += 1;
        session_id
    }

    pub(crate) fn resolve_action_pane_id(
        &self,
        requested_pane_id: WorkspacePaneId,
    ) -> Option<WorkspacePaneId> {
        if self.pane_can_accept_workspace_action(requested_pane_id) {
            Some(requested_pane_id)
        } else if self.pane_can_accept_workspace_action(self.focused_pane) {
            Some(self.focused_pane)
        } else {
            first_workspace_leaf_id(&self.root).or_else(|| {
                if self.terminal_sessions.is_empty() {
                    workspace_empty_root_leaf_id(&self.root)
                } else {
                    None
                }
            })
        }
    }
}
