use super::*;
use crate::*;

impl WorkspaceModel {
    pub(crate) fn leaf_order(&self) -> Vec<WorkspacePaneId> {
        let mut pane_ids = Vec::new();
        collect_workspace_leaf_ids(&self.root, &mut pane_ids);
        pane_ids
    }

    pub(crate) fn rendered_leaf_order(&self) -> Vec<WorkspacePaneId> {
        if let Some(pane_id) = self.focus_mode_pane
            && self.find_leaf(pane_id).is_some()
        {
            return vec![pane_id];
        }
        self.leaf_order()
    }

    pub(crate) fn toggle_focus_mode_from_tab_double_click(
        &mut self,
        pane_id: WorkspacePaneId,
        session_id: TerminalSessionId,
    ) -> bool {
        /*
        CDXC:FocusMode 2026-06-22-17:00:
        Agents workspace tab double-click parity must select and focus the clicked tab group before changing Focus mode. The first double-click zooms that pane when it has a visible non-sleeping placeholder, a second double-click exits, and sleeping-only panes remain selected/focused without waking or materializing terminals.

        CDXC:FocusMode 2026-06-22-17:00:
        If this helper is invoked for a different pane while Focus mode is active, keep the existing toggle semantics: select the requested tab first, then clear Focus mode instead of jumping directly into a different zoomed pane.
        */
        if !self.session_belongs_to_pane(pane_id, session_id) {
            return false;
        }

        let focused_pane_before = self.focused_pane;
        let active_session_before = self
            .find_leaf(pane_id)
            .and_then(|leaf| leaf.tab_group.active_session_id());
        let focus_mode_pane_before = self.focus_mode_pane;

        self.select_tab(pane_id, session_id);
        let focus_mode_toggled = self.toggle_focus_mode();

        focus_mode_toggled
            || focused_pane_before != self.focused_pane
            || active_session_before != Some(session_id)
            || focus_mode_pane_before != self.focus_mode_pane
    }

    pub(crate) fn toggle_focus_mode(&mut self) -> bool {
        /*
        CDXC:FocusMode 2026-06-22-06:02:
        Agents Focus mode is a reversible in-memory zoom of the focused rendered tab group. It stores only the focused pane id, renders that leaf as the workspace body, and toggles back to the unmodified split tree; sleeping-only panes do not count toward Focus-mode availability.
        */
        if self.focus_mode_pane.take().is_some() {
            return true;
        }

        if self.focus_mode_eligible_leaf_count() <= 1
            || !self.leaf_is_focus_mode_eligible(self.focused_pane)
        {
            return false;
        }

        self.focus_mode_pane = Some(self.focused_pane);
        true
    }

    pub(crate) fn focus_mode_eligible_leaf_count(&self) -> usize {
        self.leaf_order()
            .into_iter()
            .filter(|pane_id| self.leaf_is_focus_mode_eligible(*pane_id))
            .count()
    }

    pub(crate) fn leaf_is_focus_mode_eligible(&self, pane_id: WorkspacePaneId) -> bool {
        let Some(leaf) = self.find_leaf(pane_id) else {
            return false;
        };
        leaf.tab_group.tabs.iter().any(|tab| {
            self.session(tab.session_id)
                .is_some_and(|session| session.presentation_state.counts_as_focus_mode_visible())
        })
    }

    pub(crate) fn clear_focus_mode_if_invalid(&mut self) {
        if self
            .focus_mode_pane
            .is_some_and(|pane_id| self.find_leaf(pane_id).is_none())
        {
            self.focus_mode_pane = None;
        }
    }

    pub(crate) fn normalize_workspace_tree(&mut self) -> bool {
        /*
        CDXC:Workarea 2026-07-08:
        Agents split layout normalization is a model invariant, not render filtering. Prune tabs whose shell sessions no longer exist, collapse empty leaves by unwrapping their split branch, repair stale active/focus ids, and keep the single empty leaf only for the whole-empty workspace baseline.
        */
        let valid_session_ids = self
            .terminal_sessions
            .iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>();

        if valid_session_ids.is_empty() {
            let pane_id = self.focused_pane;
            let mut changed = !workspace_node_is_empty_leaf_for_pane(&self.root, pane_id);
            if self.focus_mode_pane.take().is_some() {
                changed = true;
            }
            self.root = workspace_empty_leaf_node(pane_id);
            return changed;
        }

        let focus_replacement =
            workspace_close_focus_replacement_leaf_id(&self.root, self.focused_pane);
        let mut changed = false;
        let root = std::mem::replace(&mut self.root, workspace_dummy_node());
        self.root = normalize_workspace_node(root, &valid_session_ids, &mut changed)
            .unwrap_or_else(|| {
                changed = true;
                workspace_leaf_node_from_session_ids(self.focused_pane, self.terminal_session_ids())
            });
        changed |= self.append_unassigned_terminal_sessions_to_workspace();

        let focused_leaf_has_tabs = self
            .find_leaf(self.focused_pane)
            .is_some_and(|leaf| !leaf.tab_group.tabs.is_empty());
        if !focused_leaf_has_tabs
            && let Some(next_focus) = focus_replacement
                .filter(|pane_id| {
                    self.find_leaf(*pane_id)
                        .is_some_and(|leaf| !leaf.tab_group.tabs.is_empty())
                })
                .or_else(|| self.most_recent_pane_where(|leaf| !leaf.tab_group.tabs.is_empty()))
                .or_else(|| first_workspace_leaf_id(&self.root))
            && self.focused_pane != next_focus
        {
            self.set_focused_pane(next_focus);
            changed = true;
        }
        self.prune_pane_focus_history();

        let focus_mode_before = self.focus_mode_pane;
        self.clear_focus_mode_if_invalid();
        changed || self.focus_mode_pane != focus_mode_before
    }

    pub(crate) fn append_unassigned_terminal_sessions_to_workspace(&mut self) -> bool {
        let mut assigned_session_ids = Vec::new();
        collect_workspace_node_session_ids(&self.root, &mut assigned_session_ids);
        let assigned_session_ids = assigned_session_ids.into_iter().collect::<HashSet<_>>();
        let unassigned_session_ids = self
            .terminal_sessions
            .iter()
            .map(|session| session.id)
            .filter(|session_id| !assigned_session_ids.contains(session_id))
            .collect::<Vec<_>>();

        if unassigned_session_ids.is_empty() {
            return false;
        }

        let target_pane_id = self
            .resolve_action_pane_id(self.focused_pane)
            .or_else(|| first_workspace_leaf_id(&self.root))
            .unwrap_or(self.focused_pane);
        let Some(target_leaf) = self.find_leaf_mut(target_pane_id) else {
            self.root =
                workspace_leaf_node_from_session_ids(target_pane_id, unassigned_session_ids);
            self.set_focused_pane(target_pane_id);
            self.focus_mode_pane = None;
            return true;
        };

        let active_tab_is_valid = target_leaf
            .tab_group
            .tabs
            .iter()
            .any(|tab| tab.session_id == target_leaf.tab_group.active_tab);
        target_leaf.tab_group.tabs.extend(
            unassigned_session_ids
                .into_iter()
                .map(|session_id| WorkspaceTab { session_id }),
        );
        if !active_tab_is_valid && let Some(first_tab) = target_leaf.tab_group.tabs.first() {
            target_leaf.tab_group.active_tab = first_tab.session_id;
        }
        true
    }

    pub(crate) fn find_leaf_mut(&mut self, pane_id: WorkspacePaneId) -> Option<&mut WorkspaceLeaf> {
        find_workspace_leaf_mut(&mut self.root, pane_id)
    }

    pub(crate) fn find_leaf(&self, pane_id: WorkspacePaneId) -> Option<&WorkspaceLeaf> {
        find_workspace_leaf(&self.root, pane_id)
    }

    pub(crate) fn split_ratio(&self, split_id: WorkspaceSplitId) -> Option<f32> {
        find_workspace_split(&self.root, split_id).map(|split| workspace_split_ratio(split.ratio))
    }

    pub(crate) fn set_split_ratio(&mut self, split_id: WorkspaceSplitId, ratio: f32) -> bool {
        let next_ratio = workspace_split_ratio(ratio);
        let Some(split) = find_workspace_split_mut(&mut self.root, split_id) else {
            return false;
        };

        if (workspace_split_ratio(split.ratio) - next_ratio).abs() < 0.001 {
            return false;
        }

        split.ratio = next_ratio;
        true
    }

    /// CDXC:Workarea 2026-09-09 DECISION:
    /// User: double-clicking either divider between three side-by-side or stacked Agents panes makes all three equal in width or height.
    /// This replaces resetting only the clicked split to its saved default ratio.
    pub(crate) fn equalize_split_panes(
        &mut self,
        split_id: WorkspaceSplitId,
        layout_metrics: &HashMap<WorkspaceSplitId, SplitResizeMetrics>,
    ) -> bool {
        let Some(group_id) = workspace_split_resize_group_id(&self.root, split_id, None) else {
            return false;
        };
        let Some(metrics) = layout_metrics.get(&group_id) else {
            return false;
        };
        let Some(split) = find_workspace_split_mut(&mut self.root, group_id) else {
            return false;
        };
        equalize_workspace_split_group(split, metrics.content_span)
    }

    pub(crate) fn split_drag_ratio_bounds(
        &self,
        split_id: WorkspaceSplitId,
        content_span: f32,
    ) -> Option<(f32, f32)> {
        let split = find_workspace_split(&self.root, split_id)?;
        let minimum = split_pane_resize_minimum_for_axis(split.axis);
        split_drag_ratio_bounds_from_minimums(
            workspace_node_axis_pane_count(&split.first, split.axis) as f32 * minimum,
            workspace_node_axis_pane_count(&split.second, split.axis) as f32 * minimum,
            content_span,
        )
    }

    pub(crate) fn pane_tab_count(&self, pane_id: WorkspacePaneId) -> Option<usize> {
        self.find_leaf(pane_id)
            .map(|leaf| leaf.tab_group.tabs.len())
    }

    pub(crate) fn workspace_tab_body_drop_is_single_tab_own_pane_noop(
        &self,
        source_pane_id: WorkspacePaneId,
        target_pane_id: WorkspacePaneId,
    ) -> bool {
        source_pane_id == target_pane_id
            && self.pane_tab_count(source_pane_id).unwrap_or_default() <= 1
    }

    pub(crate) fn workspace_tab_edge_drop_is_single_tab_own_pane_noop(
        &self,
        source_pane_id: WorkspacePaneId,
        target_pane_id: WorkspacePaneId,
        zone: WorkspaceDropZone,
    ) -> bool {
        !matches!(zone, WorkspaceDropZone::Center)
            && self
                .workspace_tab_body_drop_is_single_tab_own_pane_noop(source_pane_id, target_pane_id)
    }

    pub(crate) fn pane_can_accept_workspace_action(&self, pane_id: WorkspacePaneId) -> bool {
        self.find_leaf(pane_id)
            .is_some_and(|leaf| !leaf.tab_group.tabs.is_empty())
            || (self.terminal_sessions.is_empty()
                && workspace_empty_root_leaf_id(&self.root) == Some(pane_id))
    }
}
