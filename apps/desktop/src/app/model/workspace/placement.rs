use super::*;
use crate::*;

impl WorkspaceModel {
    pub(crate) fn add_mounting_session_to_pane(
        &mut self,
        requested_pane_id: WorkspacePaneId,
    ) -> Option<TerminalSessionId> {
        /*
        CDXC:Terminal 2026-06-22-23:33:
        New Agents terminal tabs are selected shell-owned Mounting sessions until a real terminal runtime has started. The tab, pane focus, and shell id are created immediately for layout parity, but no fake Running state, libghostty mount, process launch, command text, stdout/stderr, terminal content, or runtime id persistence is allowed.

        CDXC:FocusMode 2026-07-25:
        Cmd+T and the clicked-pane new-terminal control share this model
        mutation. Tab position is stable and user-owned, so a new terminal is
        appended to the end of the target pane's tab strip instead of being
        spliced in after the active tab.
        */
        let pane_id = self.resolve_action_pane_id(requested_pane_id)?;
        let session_id = self.allocate_session_id();
        self.terminal_sessions.push(TerminalSession::placeholder(
            session_id,
            terminal_session_title_for_id(session_id),
            TerminalSessionPresentationState::Mounting,
        ));

        let Some(leaf) = self.find_leaf_mut(pane_id) else {
            self.terminal_sessions
                .retain(|session| session.id != session_id);
            return None;
        };
        let insertion_index = leaf.tab_group.tabs.len();
        leaf.tab_group
            .insert_session_at(WorkspaceTab { session_id }, insertion_index);
        leaf.tab_group.active_tab = session_id;
        self.set_focused_pane(pane_id);
        Some(session_id)
    }

    pub(crate) fn add_running_session_to_pane(
        &mut self,
        requested_pane_id: WorkspacePaneId,
        title: String,
        agent_icon: Option<&'static str>,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        /*
        CDXC:FocusRouting 2026-06-27-13:25:
        Local gxserver sidebar session attach is not a new-terminal startup placeholder. Create the selected Agents tab as Running immediately so the normal visible Ghostty mount-slot path can attach the daemon session without showing the Mounting card or persisting a fake pending state.
        */
        let pane_id = self.resolve_action_pane_id(requested_pane_id)?;
        let session_id = self.allocate_session_id();
        self.terminal_sessions.push(
            TerminalSession::placeholder(
                session_id,
                title,
                TerminalSessionPresentationState::Running,
            )
            .with_agent_icon(agent_icon),
        );

        let Some(leaf) = self.find_leaf_mut(pane_id) else {
            self.terminal_sessions
                .retain(|session| session.id != session_id);
            return None;
        };
        let insertion_index = leaf.tab_group.tabs.len();
        leaf.tab_group
            .insert_session_at(WorkspaceTab { session_id }, insertion_index);
        leaf.tab_group.active_tab = session_id;
        self.set_focused_pane(pane_id);
        Some((pane_id, session_id))
    }

    pub(crate) fn split_mounting_session_to_right_of_pane(
        &mut self,
        requested_pane_id: WorkspacePaneId,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        self.split_mounting_session_adjacent_to_pane(
            requested_pane_id,
            WorkspaceSplitAxis::Horizontal,
            false,
        )
    }

    pub(crate) fn split_mounting_session_below_pane(
        &mut self,
        requested_pane_id: WorkspacePaneId,
    ) -> Option<(WorkspacePaneId, TerminalSessionId)> {
        self.split_mounting_session_adjacent_to_pane(
            requested_pane_id,
            WorkspaceSplitAxis::Vertical,
            false,
        )
    }

    pub(crate) fn place_existing_session_for_new_terminal(
        &mut self,
        source_pane_id: WorkspacePaneId,
        requested_pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
        placement: AgentsWorkspaceNewTerminalPlacement,
    ) -> Option<WorkspacePaneId> {
        /*
        CDXC:CommandPane 2026-08-07:
        Remote gxserver presentation can publish a newly-created session before
        its SSH attach plan returns. Reconciliation necessarily gives that row
        a temporary tab owner, but quick-create placement is still owned by the
        initiating Agents action. Move the existing shell tab into the captured
        tab/split/bottom-row destination before arming its attach payload so the
        presentation race cannot turn Cmd+D, Cmd+Shift+D, or pane controls into
        ordinary tabs.
        */
        let target_pane_id = self.resolve_action_pane_id(requested_pane_id)?;
        let placed = match placement {
            AgentsWorkspaceNewTerminalPlacement::Tab => {
                self.group_tab_into_pane(source_pane_id, target_pane_id, session_id)
            }
            AgentsWorkspaceNewTerminalPlacement::SplitRight => self.split_tab_to_pane(
                source_pane_id,
                target_pane_id,
                session_id,
                WorkspaceDropZone::Right,
            ),
            AgentsWorkspaceNewTerminalPlacement::SplitBelow => self.split_tab_to_pane(
                source_pane_id,
                target_pane_id,
                session_id,
                WorkspaceDropZone::Bottom,
            ),
            AgentsWorkspaceNewTerminalPlacement::BottomRow => {
                return self.move_tab_to_bottom_row(source_pane_id, session_id);
            }
        };
        placed.then_some(self.focused_pane)
    }

    pub(crate) fn move_tab_to_bottom_row(
        &mut self,
        source_pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
    ) -> Option<WorkspacePaneId> {
        if !self.has_session(session_id) || collect_workspace_tab_count(&self.root) <= 1 {
            return None;
        }
        let (tab, source_is_empty) = self.remove_tab_for_move(source_pane_id, session_id)?;
        if source_is_empty {
            self.collapse_empty_leaf(source_pane_id);
        }
        self.clear_focus_mode_if_invalid();

        let pane_id = self.allocate_pane_id();
        let split_id = self.allocate_split_id();
        let new_leaf = WorkspaceNode::Leaf(WorkspaceLeaf {
            pane_id,
            tab_group: WorkspaceTabGroup {
                tabs: vec![tab],
                active_tab: session_id,
            },
        });
        let current_root = std::mem::replace(&mut self.root, workspace_dummy_node());
        self.root = WorkspaceNode::Split(WorkspaceSplit {
            id: split_id,
            axis: WorkspaceSplitAxis::Vertical,
            ratio: workspace_split_ratio(WORKSPACE_BOTTOM_ROW_TOP_RATIO),
            default_ratio: workspace_split_ratio(WORKSPACE_BOTTOM_ROW_TOP_RATIO),
            first: Box::new(current_root),
            second: Box::new(new_leaf),
        });
        self.set_focused_pane(pane_id);
        self.focus_mode_pane = None;
        self.normalize_workspace_tree();
        Some(pane_id)
    }

    pub(crate) fn append_mounting_session_bottom_row(
        &mut self,
    ) -> (WorkspacePaneId, TerminalSessionId) {
        /*
        CDXC:Terminal 2026-06-22-23:33:
        Full-width secondary terminal creation must append below the whole Agents workspace, not split the clicked pane. Keep the existing split/tab tree intact as the top branch, create a new bottom-row leaf with one selected Mounting terminal, focus that leaf, and clear Agents Focus mode so the row is visible without creating fake Running state, command-pane sessions, processes, libghostty surfaces, command text, stdout/stderr, or terminal content.
        */
        let session_id = self.allocate_session_id();
        let pane_id = self.allocate_pane_id();
        let split_id = self.allocate_split_id();
        self.terminal_sessions.push(TerminalSession::placeholder(
            session_id,
            terminal_session_title_for_id(session_id),
            TerminalSessionPresentationState::Mounting,
        ));
        let new_leaf = WorkspaceNode::Leaf(WorkspaceLeaf {
            pane_id,
            tab_group: WorkspaceTabGroup {
                tabs: vec![WorkspaceTab { session_id }],
                active_tab: session_id,
            },
        });
        let current_root = std::mem::replace(&mut self.root, workspace_dummy_node());
        self.root = WorkspaceNode::Split(WorkspaceSplit {
            id: split_id,
            axis: WorkspaceSplitAxis::Vertical,
            ratio: workspace_split_ratio(WORKSPACE_BOTTOM_ROW_TOP_RATIO),
            default_ratio: workspace_split_ratio(WORKSPACE_BOTTOM_ROW_TOP_RATIO),
            first: Box::new(current_root),
            second: Box::new(new_leaf),
        });
        self.set_focused_pane(pane_id);
        self.focus_mode_pane = None;
        self.normalize_workspace_tree();
        (pane_id, session_id)
    }

    pub(crate) fn merge_all_tabs_into_pane(&mut self, requested_pane_id: WorkspacePaneId) -> bool {
        /*
        CDXC:CommandPane 2026-06-22-13:17:
        Merge All Tabs collapses the Agents workspace split root into the clicked or focused pane id while preserving every existing Agents terminal tab/session id and presentation state in tree-render order. Single-pane layouts no-op; multi-pane merges clear Focus mode because the pane geometry no longer exists, and command-pane sessions are intentionally outside this model.
        */
        let Some(target_pane_id) = self.resolve_action_pane_id(requested_pane_id) else {
            return false;
        };
        let leaf_order = self.leaf_order();
        if leaf_order.len() <= 1 {
            return false;
        }

        let target_active_session = self
            .find_leaf(target_pane_id)
            .and_then(|leaf| leaf.tab_group.active_session_id());
        let fallback_active_session = leaf_order.iter().find_map(|pane_id| {
            self.find_leaf(*pane_id)
                .and_then(|leaf| leaf.tab_group.active_session_id())
        });
        let mut tabs = Vec::new();
        collect_workspace_tabs_in_tree_order(&self.root, &mut tabs);
        tabs.retain(|tab| self.has_session(tab.session_id));
        if tabs.is_empty() {
            return false;
        }

        let active_tab = target_active_session
            .or(fallback_active_session)
            .filter(|session_id| tabs.iter().any(|tab| tab.session_id == *session_id))
            .unwrap_or(tabs[0].session_id);
        self.root = WorkspaceNode::Leaf(WorkspaceLeaf {
            pane_id: target_pane_id,
            tab_group: WorkspaceTabGroup { tabs, active_tab },
        });
        self.set_focused_pane(target_pane_id);
        self.focus_mode_pane = None;
        self.normalize_workspace_tree();
        true
    }
}
