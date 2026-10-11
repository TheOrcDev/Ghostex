//! Which of the command pane's two trees a command group lives in, and what that means for
//! whether its terminals are on screen.

use crate::*;

/// CDXC:CommandPane 2026-09-22 DECISION:
/// User: "add the ability to open a Terminal as a view", one that "should work just like command
/// panes but be on the right side instead", with "its own tabs bar like command pane", and "only
/// allow 1 terminal view there". The Terminal view is therefore a second tree of command groups
/// inside the one `CommandPaneModel`, drawn inside the view panel instead of below or beside the
/// workspace. Both trees share one session list and one id space, so daemon mappings, Action
/// ownership, Delayed Send timers, rename, the sidebar indicators and tab drags between the two
/// keep working without a second model; only where a group is drawn, and when it is on screen,
/// depends on its dock. F12 and the header toggle stay with the Commands pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CommandPaneDock {
    /// The Commands pane: the bottom or right dock next to the workspace.
    Panel,
    /// The Terminal view: a tab of the view panel.
    View,
}

impl CommandPaneModel {
    pub(crate) fn root_for_dock(&self, dock: CommandPaneDock) -> &CommandPaneNode {
        match dock {
            CommandPaneDock::Panel => &self.root,
            CommandPaneDock::View => &self.view_root,
        }
    }

    pub(crate) fn root_for_dock_mut(&mut self, dock: CommandPaneDock) -> &mut CommandPaneNode {
        match dock {
            CommandPaneDock::Panel => &mut self.root,
            CommandPaneDock::View => &mut self.view_root,
        }
    }

    pub(crate) fn dock_for_group(&self, group_id: CommandPaneGroupId) -> Option<CommandPaneDock> {
        if command_node_contains_group(&self.root, group_id) {
            Some(CommandPaneDock::Panel)
        } else if command_node_contains_group(&self.view_root, group_id) {
            Some(CommandPaneDock::View)
        } else {
            None
        }
    }

    /// The tree that holds `group_id`, for the tree mutations that take a root.
    pub(crate) fn root_for_group_mut(
        &mut self,
        group_id: CommandPaneGroupId,
    ) -> Option<&mut CommandPaneNode> {
        let dock = self.dock_for_group(group_id)?;
        Some(self.root_for_dock_mut(dock))
    }

    /// Whether a dock's terminals can be on screen right now. The Commands pane is on screen while
    /// it is expanded; the Terminal view while it is the view the panel shows, which the app
    /// writes into `view_dock_visible` at every mode change.
    pub(crate) fn dock_visible(&self, dock: CommandPaneDock) -> bool {
        match dock {
            CommandPaneDock::Panel => self.is_expanded(),
            CommandPaneDock::View => self.view_dock_visible,
        }
    }

    pub(crate) fn any_dock_visible(&self) -> bool {
        self.is_expanded() || self.view_dock_visible
    }

    pub(crate) fn group_dock_visible(&self, group_id: CommandPaneGroupId) -> bool {
        self.dock_for_group(group_id)
            .is_some_and(|dock| self.dock_visible(dock))
    }

    pub(crate) fn focused_group_dock(&self) -> Option<CommandPaneDock> {
        self.dock_for_group(self.focused_group)
    }

    /// The gate every focused-session route reads instead of `is_expanded()`: the focused group
    /// resolves, and the dock it lives in is on screen.
    pub(crate) fn focused_group_dock_visible(&self) -> bool {
        self.group_dock_visible(self.focused_group)
    }

    pub(crate) fn focused_group_in_panel(&self) -> bool {
        self.focused_group_dock() == Some(CommandPaneDock::Panel)
    }

    pub(crate) fn has_panel_sessions(&self) -> bool {
        command_node_leaf_count(&self.root) > 0
    }

    pub(crate) fn has_view_sessions(&self) -> bool {
        command_node_leaf_count(&self.view_root) > 0
    }

    pub(crate) fn first_leaf_id_in_dock(
        &self,
        dock: CommandPaneDock,
    ) -> Option<CommandPaneGroupId> {
        first_command_leaf_id(self.root_for_dock(dock))
    }

    /// The Commands pane's own tabs, for the collapsed strip and the layout plan, which describe
    /// only that dock.
    pub(crate) fn panel_flat_tab_ids(&self) -> Vec<(CommandPaneGroupId, CommandSessionId)> {
        let mut tabs = Vec::new();
        collect_command_tabs(&self.root, &mut tabs);
        tabs
    }

    /// The focused group's active tab if the focused group is in `dock`, else the dock's first tab.
    pub(crate) fn active_group_and_session_id_in_dock(
        &self,
        dock: CommandPaneDock,
    ) -> Option<(CommandPaneGroupId, CommandSessionId)> {
        self.find_leaf(self.focused_group)
            .filter(|_| self.focused_group_dock() == Some(dock))
            .and_then(|leaf| leaf.tab_group.active_session_id())
            .map(|session_id| (self.focused_group, session_id))
            .or_else(|| {
                first_command_leaf(self.root_for_dock(dock)).and_then(|leaf| {
                    leaf.tab_group
                        .active_session_id()
                        .map(|session_id| (leaf.group_id, session_id))
                })
            })
    }

    /// Creating a tab in the Commands pane expands it. Creating one in the Terminal view changes
    /// nothing here: the view is on screen exactly when it is the active view tab, which the app
    /// decides.
    pub(crate) fn reveal_dock_for_group(&mut self, group_id: CommandPaneGroupId) {
        if self.dock_for_group(group_id) == Some(CommandPaneDock::Panel) {
            self.expand();
        }
    }

    /// Opening the Terminal view: focus its existing tab, or create its first `Command Terminal`
    /// the way F12 creates the Commands pane's first one. Returns the tab and whether it is new.
    pub(crate) fn ensure_view_session_for_open(
        &mut self,
    ) -> Option<(CommandPaneGroupId, CommandSessionId, bool)> {
        if let Some((group_id, session_id)) =
            self.active_group_and_session_id_in_dock(CommandPaneDock::View)
        {
            self.focused_group = group_id;
            return Some((group_id, session_id, false));
        }
        let session_id = self.allocate_session_id();
        self.terminal_sessions
            .push(CommandTerminalSession::placeholder(
                session_id,
                COMMAND_PANE_DEFAULT_SESSION_TITLE.to_string(),
            ));
        let tab = CommandPaneTab { session_id };
        let group_id = self.replace_empty_command_layout_with_created_tab(
            CommandPaneDock::View,
            tab,
            session_id,
        );
        Some((group_id, session_id, true))
    }

    /// Both trees are gone: the model is back to its empty shape.
    pub(crate) fn reset_empty_layout(&mut self) {
        self.root = command_pane_dummy_node();
        self.view_root = command_pane_dummy_node();
        self.focused_group = CommandPaneGroupId(0);
        self.focus_mode_group = None;
        self.collapse();
    }

    /// After a tree lost a leaf: an emptied Commands pane hides like it always did, and an emptied
    /// Terminal view tree drops back to the dummy leaf so the app can close its tab.
    pub(crate) fn prune_emptied_docks(&mut self) {
        if !self.has_panel_sessions() && !matches!(self.root, CommandPaneNode::Leaf(_)) {
            self.root = command_pane_dummy_node();
        }
        if !self.has_panel_sessions() && self.is_expanded() {
            self.collapse();
        }
        if !self.has_view_sessions() {
            self.view_root = command_pane_dummy_node();
        }
    }
}

impl CommandPaneModel {
    /// The leaf Focus mode zooms inside `dock`, if Focus mode is on there and still valid. Each dock
    /// renders either this leaf alone or its whole tree.
    pub(crate) fn focus_mode_leaf_for_dock(
        &self,
        dock: CommandPaneDock,
    ) -> Option<&CommandPaneLeaf> {
        let group_id = self.focus_mode_group?;
        if self.dock_for_group(group_id) != Some(dock) {
            return None;
        }
        let leaf = self.find_leaf(group_id)?;
        (self.focus_mode_eligible_group_count_for_group(group_id) > 1
            && self.group_is_focus_mode_eligible_without_focus(group_id))
        .then_some(leaf)
    }

    /// A Terminal view leaf has no fixed control cluster (Pin, Keep open and Minimize are Commands
    /// pane controls), so its tab run only reserves room for the inline add button.
    pub(crate) fn view_leaf_tab_add_visible_for_chrome_width(chrome_width: f32) -> bool {
        chrome_width
            >= COMMAND_PANE_MINIMUM_VISIBLE_TAB_VIEWPORT_WIDTH_WITH_DOUBLE_CLICK_TARGET
                + COMMAND_PANE_TAB_ADD_BUTTON_GAP
                + COMMAND_PANE_TAB_BAR_HEIGHT
    }

    pub(crate) fn view_leaf_sticky_active_tab_trailing_inset(tab_add_visible: bool) -> f32 {
        if tab_add_visible {
            COMMAND_PANE_TAB_ADD_BUTTON_GAP + COMMAND_PANE_TAB_BAR_HEIGHT
        } else {
            0.0
        }
    }
}
