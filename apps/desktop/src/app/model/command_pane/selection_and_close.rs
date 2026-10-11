use super::*;
use crate::*;

impl CommandPaneModel {
    pub(crate) fn set_focused_group_for_selected_owner(&mut self, group_id: CommandPaneGroupId) {
        /*
        CDXC:FocusMode 2026-06-26-04:43:
        Command-pane Focus is a visibility filter, not sticky selection. When a command insertion, reorder, or grouping path makes a different command group the active owner, clear Focus before render so the selected command owner is visible; same-group selection and reorder keep Focus reversible.
        */
        self.focused_group = group_id;
        if self
            .focus_mode_group
            .is_some_and(|focus_group_id| focus_group_id != group_id)
        {
            self.focus_mode_group = None;
        }
    }

    pub(crate) fn select_session_in_group(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> bool {
        let selected = self.find_leaf_mut(group_id).is_some_and(|leaf| {
            if leaf.tab_group.has_session(session_id) {
                leaf.tab_group.active_session = session_id;
                true
            } else {
                false
            }
        });

        if selected {
            self.set_focused_group_for_selected_owner(group_id);
            self.clear_focus_mode_if_invalid();
        }
        selected
    }

    pub(crate) fn select_session_in_group_for_hidden_open(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        content_height: f32,
        default_height_px: f32,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-06-25-12:10:
        Selecting a tab from collapsed command-strip chrome is a hidden-open path like macOS `openCommandsPanelForActiveProject`. Apply the Workspace default-height reset before expanding so collapsed tab clicks and context-menu Select do not preserve a stale hidden pane height.
        */
        if !self.select_session_in_group(group_id, session_id) {
            return false;
        }

        self.prepare_hidden_open_with_default_height_px(content_height, default_height_px);
        self.expand();
        true
    }

    pub(crate) fn focus_group(&mut self, group_id: CommandPaneGroupId) -> bool {
        if self.find_leaf(group_id).is_some() {
            self.set_focused_group_for_selected_owner(group_id);
            true
        } else {
            false
        }
    }

    pub(crate) fn toggle_focus_mode_for_tab(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> bool {
        /*
        CDXC:FocusMode 2026-06-25-21:40:
        Command-tab Focus is the command-panel equivalent of native split-owner Focus mode. Store only the focused command group id, preserve the full command split tree for reversible exit, and select/focus the clicked command tab before zooming so the visible owner matches the menu target without creating terminal processes, command text, fallback rows, overlays, or persisted runtime state.
        */
        if self.focus_mode_group == Some(group_id) {
            self.focus_mode_group = None;
            return self.select_session_in_group(group_id, session_id);
        }

        if !self.tab_context_allows_focus_mode(group_id, session_id) {
            return false;
        }
        if !self.select_session_in_group(group_id, session_id) {
            return false;
        }
        self.focus_mode_group = Some(group_id);
        true
    }

    pub(crate) fn acknowledge_attention_for_session_activation(
        &mut self,
        session_id: CommandSessionId,
    ) -> bool {
        /*
        CDXC:Notifications 2026-06-25-19:58:
        Native command content, titlebar, and tab activation acknowledge a focused Attention command session. GPUI should clear only the directly activated command session from Attention to Idle, leaving Working, Delayed Send flags, sleeping state, and Agents workspace activity unchanged.
        */
        let Some(session) = self.session_mut(session_id) else {
            return false;
        };
        if session.activity != CommandTerminalActivity::Attention {
            return false;
        }
        session.activity = CommandTerminalActivity::Idle;
        true
    }

    pub(crate) fn acknowledge_attention_for_focused_session_activation(&mut self) -> bool {
        /*
        CDXC:Notifications 2026-06-26-00:38:
        Responder and titlebar-control command activation must acknowledge only the active tab in the live focused command group. A stale `focused_group` is not an activation target and must no-op instead of falling back to the first command group and clearing unrelated Attention state.
        */
        let Some((_group_id, session_id)) = self.focused_group_active_session_id() else {
            return false;
        };
        self.acknowledge_attention_for_session_activation(session_id)
    }

    pub(crate) fn acknowledge_attention_for_live_focused_group_activation(&mut self) -> bool {
        /*
        CDXC:Notifications 2026-06-25-23:55:
        Keyboard focus transfer into an already-open command panel is responder-like. Acknowledge only the active session in the live `focused_group`; stale command focus must not fall back to the first command group and clear unrelated Attention state.
        */
        let Some((_group_id, session_id)) = self.focused_group_active_session_id() else {
            return false;
        };
        self.acknowledge_attention_for_session_activation(session_id)
    }

    pub(crate) fn cycle_active_session(&mut self, reverse: bool) -> bool {
        /*
        CDXC:FocusRouting 2026-06-25-23:20:
        Command-pane Ctrl-Tab parity is responder-like: cycle only the live focused command group and no-op when `focused_group` is stale instead of falling back to the first command group.
        */
        self.find_leaf_mut(self.focused_group)
            .and_then(|leaf| leaf.tab_group.cycle_active_session(reverse))
            .is_some()
    }

    pub(crate) fn close_session(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> bool {
        let Some((_tab, source_is_empty)) = self.remove_tab_for_move(group_id, session_id) else {
            return false;
        };

        self.terminal_sessions
            .retain(|session| session.id != session_id);

        if self.terminal_sessions.is_empty() {
            self.reset_empty_layout();
            return true;
        }

        if source_is_empty {
            self.collapse_empty_leaf(group_id);
        }
        self.clear_focus_mode_if_invalid();
        true
    }

    pub(crate) fn close_session_from_direct_tab_close(
        &mut self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-06-26-06:18:
        Direct GPUI command-tab close mirrors native command titlebar close: select the clicked command session before removal so right-then-left neighbor selection is resolved from the close target, not from a previously active tab. Scoped Close Left/Right/Others continue to call `close_session` directly because their native menu rows do not focus the clicked terminal first.
        */
        if !self.select_session_in_group(group_id, session_id) {
            return false;
        }
        self.close_session(group_id, session_id)
    }

    pub(crate) fn tab_session_ids_for_close_scope(
        &self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        scope: CommandPaneTabCloseScope,
    ) -> Vec<CommandSessionId> {
        /*
        CDXC:CommandPane 2026-06-25-11:20:
        GPUI command-tab context menu close scopes must match macOS command-panel tab behavior: resolve the clicked tab's sibling ids before closing anything, stay inside that command group, and never cross into another command split group, Agents workspace pane, Browser tab, project-editor surface, command text, terminal output, paths, or persisted shell inference.
        */
        let Some(leaf) = self.find_leaf(group_id) else {
            return Vec::new();
        };
        let tab_session_ids = leaf
            .tab_group
            .tabs
            .iter()
            .map(|tab| tab.session_id)
            .collect::<Vec<_>>();
        let Some(tab_index) = tab_session_ids
            .iter()
            .position(|candidate| *candidate == session_id)
        else {
            return Vec::new();
        };

        match scope {
            CommandPaneTabCloseScope::Close => vec![session_id],
            CommandPaneTabCloseScope::CloseLeft => tab_session_ids[..tab_index].to_vec(),
            CommandPaneTabCloseScope::CloseOthers => tab_session_ids
                .into_iter()
                .filter(|candidate| *candidate != session_id)
                .collect(),
            CommandPaneTabCloseScope::CloseRight => tab_session_ids[tab_index + 1..].to_vec(),
        }
    }

    pub(crate) fn tab_session_ids_for_sleep_scope(
        &self,
        group_id: CommandPaneGroupId,
        session_id: CommandSessionId,
        scope: CommandPaneTabSleepScope,
    ) -> Vec<CommandSessionId> {
        /*
        CDXC:SessionSleep 2026-06-25-14:27:
        Command-tab Sleep scopes use the clicked command group's sibling list, just like native pane-tab sleep. Keep sleep resolution inside the command group so Sleep Right/Left/Others never crosses command splits, Agents workspace panes, Browser tabs, project-editor surfaces, command text, terminal output, paths, or persisted shell inference.
        */
        let Some(leaf) = self.find_leaf(group_id) else {
            return Vec::new();
        };
        let tab_session_ids = leaf
            .tab_group
            .tabs
            .iter()
            .map(|tab| tab.session_id)
            .collect::<Vec<_>>();
        let Some(tab_index) = tab_session_ids
            .iter()
            .position(|candidate| *candidate == session_id)
        else {
            return Vec::new();
        };

        match scope {
            CommandPaneTabSleepScope::Sleep => vec![session_id],
            CommandPaneTabSleepScope::SleepLeft => tab_session_ids[..tab_index].to_vec(),
            CommandPaneTabSleepScope::SleepOthers => tab_session_ids
                .into_iter()
                .filter(|candidate| *candidate != session_id)
                .collect(),
            CommandPaneTabSleepScope::SleepRight => tab_session_ids[tab_index + 1..].to_vec(),
        }
    }

    pub(crate) fn set_session_sleeping(
        &mut self,
        session_id: CommandSessionId,
        is_sleeping: bool,
    ) -> bool {
        let Some(session) = self.session_mut(session_id) else {
            return false;
        };
        if session.is_sleeping == is_sleeping {
            return false;
        }
        session.is_sleeping = is_sleeping;
        if is_sleeping {
            /*
            CDXC:DelayedSend 2026-06-25-15:46:
            Manual command-tab Sleep parks the terminal surface but must not cancel Delayed Send or Close After Done intent. Native keeps those timers/flags attached to the session; Delayed Send fires only if the tab is awake again at the deadline, and Close After Done resumes countdown evaluation after wake.
            */
            session.activity = CommandTerminalActivity::Idle;
            session.action_close_terminal_on_exit = false;
            session.action_run_id = None;
            session.action_status_file_path = None;
        }
        self.clear_focus_mode_if_invalid();
        true
    }
}
