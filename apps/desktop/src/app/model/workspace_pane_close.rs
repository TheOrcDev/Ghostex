//! Closing an Agents pane without closing the sessions it holds.

use crate::app::model::{
    WorkspaceModel, WorkspacePaneId, first_workspace_leaf_id,
    workspace_close_focus_replacement_leaf_id,
};

impl WorkspaceModel {
    /// The pane leaves the split and every session it held, shown or not, joins the neighbouring
    /// pane behind the session that pane is showing. Sessions keep running; only the viewport
    /// goes. Returns the pane that took them, or `None` for the last pane, which cannot close.
    pub(crate) fn close_pane_keeping_sessions(
        &mut self,
        pane_id: WorkspacePaneId,
    ) -> Option<WorkspacePaneId> {
        if self.leaf_order().len() <= 1 || self.find_leaf(pane_id).is_none() {
            return None;
        }
        let target = workspace_close_focus_replacement_leaf_id(&self.root, pane_id)
            .filter(|target| *target != pane_id && self.find_leaf(*target).is_some())
            .or_else(|| {
                self.leaf_order()
                    .into_iter()
                    .find(|candidate| *candidate != pane_id)
            })?;
        let tabs = std::mem::take(&mut self.find_leaf_mut(pane_id)?.tab_group.tabs);
        let target_leaf = self.find_leaf_mut(target)?;
        target_leaf.tab_group.tabs.extend(tabs);
        self.collapse_empty_leaf(pane_id);
        self.clear_focus_mode_if_invalid();
        let target = self
            .find_leaf(target)
            .map(|_| target)
            .or_else(|| first_workspace_leaf_id(&self.root))?;
        self.set_focused_pane(target);
        self.normalize_workspace_tree();
        Some(target)
    }
}
