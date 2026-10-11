use super::*;
use crate::*;

impl CommandPaneModel {
    pub(crate) fn reorder_tab_within_group(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        insertion_index: usize,
    ) -> bool {
        let Some((source_index, final_index)) =
            self.tab_strip_reorder_indices(group_id, session_id, insertion_index)
        else {
            return false;
        };
        if final_index == source_index {
            return false;
        }

        let Some(leaf) = self.find_leaf_mut(group_id) else {
            return false;
        };
        let active_session = leaf.tab_group.active_session;
        let Some(tab) = leaf.tab_group.remove_session(session_id) else {
            return false;
        };
        leaf.tab_group.insert_session_at(tab, final_index);
        leaf.tab_group.active_session = active_session;
        self.set_focused_group_for_selected_owner(group_id);
        true
    }

    pub(crate) fn group_tab_into_group(
        &mut self,
        source_group_id: CommandPaneGroupId,
        target_group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> bool {
        if !self.has_session(session_id) || self.find_leaf(target_group_id).is_none() {
            return false;
        }

        if source_group_id == target_group_id {
            return self.select_session_in_group(target_group_id, session_id);
        }

        let Some((tab, source_is_empty)) = self.remove_tab_for_move(source_group_id, session_id)
        else {
            return false;
        };

        if source_is_empty {
            self.collapse_empty_leaf(source_group_id);
        }

        let Some(target_leaf) = self.find_leaf_mut(target_group_id) else {
            return false;
        };
        target_leaf
            .tab_group
            .insert_session_at(tab, target_leaf.tab_group.tabs.len());
        target_leaf.tab_group.active_session = session_id;
        self.set_focused_group_for_selected_owner(target_group_id);
        self.clear_focus_mode_if_invalid();
        true
    }

    pub(crate) fn split_tab_to_group(
        &mut self,
        source_group_id: CommandPaneGroupId,
        target_group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        zone: WorkspaceDropZone,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-06-22-06:13:
        Command-pane drag/drop is intentionally narrower than Agents workspace drag/drop. Center drops group command tabs into the target command tab group, left/right edge drops create horizontal command splits, and top/bottom intent is treated as center so command panes never create vertical splits in this in-memory slice.

        CDXC:FocusMode 2026-06-26-06:37:
        Command drag/drop must match native Focus-mode ownership. Same-group grouping stays inside the focused command owner, while left/right side drops split the dragged command into a new selected owner, clear command Focus, and render that dragged command immediately.

        CDXC:CommandPane 2026-06-26-06:37:
        Native command-panel same-session body side drops resolve the drop to the first or last remaining tab sibling before removing the dragged tab. GPUI owns only command groups here, so split after removal beside the still-live source group, reject single-tab self side drops, leave the source group order/selection to the normal removal rule, and focus the new dragged split group without touching unrelated groups.
        */
        // Top and bottom split only in the Terminal view (CDXC:CommandPane 2026-09-22 in
        // drag_transfer.rs); in the Commands pane they still group, as the 06-22 rule above says.
        let Some(dock) = self.dock_for_group(target_group_id) else {
            return false;
        };
        if !command_pane_drop_zone_splits(dock, zone) {
            return self.group_tab_into_group(source_group_id, target_group_id, session_id);
        }

        if !self.has_session(session_id) || self.find_leaf(target_group_id).is_none() {
            return false;
        }

        if source_group_id == target_group_id
            && self.pane_tab_count(source_group_id).unwrap_or_default() <= 1
        {
            return false;
        }

        let Some((tab, source_is_empty)) = self.remove_tab_for_move(source_group_id, session_id)
        else {
            return false;
        };

        if source_is_empty {
            self.collapse_empty_leaf(source_group_id);
        }

        let group_id = self.allocate_group_id();
        let split_id = self.allocate_split_id();
        let new_leaf = CommandPaneLeaf {
            group_id,
            tab_group: CommandPaneTabGroup {
                tabs: vec![tab],
                active_session: session_id,
            },
        };
        let axis = command_pane_drop_zone_split_axis(zone);
        let dragged_first = matches!(zone, WorkspaceDropZone::Left | WorkspaceDropZone::Top);

        let Some(root) = self.root_for_group_mut(target_group_id) else {
            return false;
        };
        if insert_command_leaf_split(
            root,
            target_group_id,
            new_leaf,
            axis,
            dragged_first,
            split_id,
        ) {
            self.focus_mode_group = None;
            self.focused_group = group_id;
            true
        } else {
            false
        }
    }

    pub(crate) fn remove_tab_for_move(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> Option<(CommandPaneTab, bool)> {
        let leaf = self.find_leaf_mut(group_id)?;
        let tab = leaf.tab_group.remove_session(session_id)?;
        let source_is_empty = leaf.tab_group.tabs.is_empty();
        Some((tab, source_is_empty))
    }

    pub(crate) fn collapse_empty_leaf(&mut self, group_id: CommandPaneGroupId) {
        let Some(dock) = self.dock_for_group(group_id) else {
            return;
        };
        let root = self.root_for_dock_mut(dock);
        let root_is_empty = collapse_empty_command_leaf(root, group_id);
        if root_is_empty {
            *root = command_pane_dummy_node();
        }
        self.prune_emptied_docks();

        if self.focused_group == group_id
            && let Some(first_leaf_id) = self
                .first_leaf_id_in_dock(dock)
                .or_else(|| first_command_leaf_id(&self.root))
                .or_else(|| first_command_leaf_id(&self.view_root))
        {
            self.focused_group = first_leaf_id;
        }
        self.clear_focus_mode_if_invalid();
    }

    pub(crate) fn find_leaf(&self, group_id: CommandPaneGroupId) -> Option<&CommandPaneLeaf> {
        find_command_leaf(&self.root, group_id)
            .or_else(|| find_command_leaf(&self.view_root, group_id))
    }

    pub(crate) fn find_leaf_mut(
        &mut self,
        group_id: CommandPaneGroupId,
    ) -> Option<&mut CommandPaneLeaf> {
        if command_node_contains_group(&self.root, group_id) {
            find_command_leaf_mut(&mut self.root, group_id)
        } else {
            find_command_leaf_mut(&mut self.view_root, group_id)
        }
    }
}
