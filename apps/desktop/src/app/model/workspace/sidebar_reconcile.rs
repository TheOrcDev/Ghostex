use super::*;
use crate::*;

impl WorkspaceModel {
    pub(crate) fn reconcile_with_sidebar_tab_sessions(
        &mut self,
        active_project_id: Option<&str>,
        tab_sessions: &[GpuiSidebarWorkspaceTabSession],
        local_workspace_session_mappings: &mut HashMap<
            GpuiLocalWorkspaceSessionKey,
            TerminalSessionId,
        >,
        remote_attach_sessions: &mut HashMap<GpuiRemoteAttachSessionKey, TerminalSessionId>,
    ) -> bool {
        /*
        CDXC:CommandPane 2026-07-05:
        The Agents tab tree mirrors the active SidebarApp group. The sidebar
        owns filtering and order; Rust only maps projected gxserver ids to
        local shell session ids, removes tabs absent from the projection, and
        updates title/lifecycle chrome from the row payload. Existing mapped
        sessions keep their pane, while newly listed rows append to the
        focused tab group in sidebar order.
        */
        let mut changed = false;
        let keys = tab_sessions
            .iter()
            .map(|session| session.key.clone())
            .collect::<HashSet<_>>();
        let mut allowed_shell_sessions = HashSet::new();

        for tab_session in tab_sessions.iter() {
            let shell_session_id = if let Some(shell_session_id) =
                workspace_terminal_session_mapping_get(
                    &tab_session.key,
                    local_workspace_session_mappings,
                    remote_attach_sessions,
                ) {
                if self.session(shell_session_id).is_some() {
                    shell_session_id
                } else {
                    workspace_terminal_session_mapping_remove(
                        &tab_session.key,
                        local_workspace_session_mappings,
                        remote_attach_sessions,
                    );
                    let session_id = self.allocate_session_id();
                    self.terminal_sessions.push(
                        TerminalSession::placeholder(
                            session_id,
                            tab_session.title.clone(),
                            tab_session.presentation_state,
                        )
                        .with_activity(tab_session.activity)
                        .with_agent_icon(tab_session.agent_icon)
                        .with_kind(tab_session.kind),
                    );
                    workspace_terminal_session_mapping_insert(
                        tab_session.key.clone(),
                        session_id,
                        local_workspace_session_mappings,
                        remote_attach_sessions,
                    );
                    changed = true;
                    session_id
                }
            } else {
                let session_id = self.allocate_session_id();
                self.terminal_sessions.push(
                    TerminalSession::placeholder(
                        session_id,
                        tab_session.title.clone(),
                        tab_session.presentation_state,
                    )
                    .with_activity(tab_session.activity)
                    .with_agent_icon(tab_session.agent_icon)
                    .with_kind(tab_session.kind),
                );
                workspace_terminal_session_mapping_insert(
                    tab_session.key.clone(),
                    session_id,
                    local_workspace_session_mappings,
                    remote_attach_sessions,
                );
                changed = true;
                session_id
            };

            if let Some(session) = self
                .terminal_sessions
                .iter_mut()
                .find(|session| session.id == shell_session_id)
            {
                if session.title != tab_session.title {
                    session.title = tab_session.title.clone();
                    changed = true;
                }
                if session.agent_icon != tab_session.agent_icon {
                    session.agent_icon = tab_session.agent_icon;
                    changed = true;
                }
                if session.activity != tab_session.activity {
                    session.activity = tab_session.activity;
                    changed = true;
                }
                if session.is_generating_first_prompt_title
                    != tab_session.is_generating_first_prompt_title
                {
                    session.is_generating_first_prompt_title =
                        tab_session.is_generating_first_prompt_title;
                    changed = true;
                }
                if session.kind != tab_session.kind {
                    session.kind = tab_session.kind;
                    session.startup_eligible_when_mounting = false;
                    session.zmx_session_name = None;
                    changed = true;
                }
                if session.presentation_state != tab_session.presentation_state {
                    session.set_presentation_state_with_startup_eligibility(
                        tab_session.presentation_state,
                        false,
                    );
                    changed = true;
                }
            }
            allowed_shell_sessions.insert(shell_session_id);
        }

        let before_session_count = self.terminal_sessions.len();
        self.terminal_sessions
            .retain(|session| allowed_shell_sessions.contains(&session.id));
        changed |= self.terminal_sessions.len() != before_session_count;
        local_workspace_session_mappings.retain(|key, shell_session_id| {
            keys.contains(&GpuiWorkspaceTerminalSessionKey::Local(key.clone()))
                && allowed_shell_sessions.contains(shell_session_id)
        });
        remote_attach_sessions.retain(|key, shell_session_id| {
            let belongs_to_active_project = active_project_id
                == Some(
                    gpui_remote_scoped_project_id(
                        key.remote_machine_id.as_str(),
                        key.project_id.as_str(),
                    )
                    .as_str(),
                );
            !belongs_to_active_project
                || (keys.contains(&GpuiWorkspaceTerminalSessionKey::Remote(key.clone()))
                    && allowed_shell_sessions.contains(shell_session_id))
        });

        if tab_sessions.is_empty() {
            if !self.terminal_sessions.is_empty()
                || collect_workspace_tab_count(&self.root) > 0
                || !matches!(self.root, WorkspaceNode::Leaf(_))
            {
                self.terminal_sessions.clear();
                self.root = workspace_empty_leaf_node(self.focused_pane);
                self.focus_mode_pane = None;
                changed = true;
            }
            changed |= self.normalize_workspace_tree();
            return changed;
        }

        let mut assigned_shell_sessions = HashSet::new();
        for pane_id in self.leaf_order() {
            let Some(leaf) = self.find_leaf_mut(pane_id) else {
                continue;
            };
            let before_tabs = leaf.tab_group.tabs.clone();
            /*
            CDXC:CommandPane 2026-07-25:
            Tab position inside a pane is owned by the Agents workspace, not by
            the sidebar projection. The sidebar reorders its rows as sessions
            report activity, so re-sorting mounted tabs by that projection made
            the tab strip shuffle on every agent turn and discarded the user's
            own drag reordering. Reconcile now only drops tabs whose session
            left the projection; surviving tabs keep their persisted index and
            newly listed sessions append below in sidebar order.
            */
            leaf.tab_group
                .tabs
                .retain(|tab| allowed_shell_sessions.contains(&tab.session_id));
            for tab in &leaf.tab_group.tabs {
                assigned_shell_sessions.insert(tab.session_id);
            }
            if !leaf
                .tab_group
                .tabs
                .iter()
                .any(|tab| tab.session_id == leaf.tab_group.active_tab)
            {
                let next_active_tab = leaf
                    .tab_group
                    .tabs
                    .first()
                    .map(|tab| tab.session_id)
                    .unwrap_or(TerminalSessionId(0));
                if leaf.tab_group.active_tab != next_active_tab {
                    leaf.tab_group.active_tab = next_active_tab;
                    changed = true;
                }
            }
            changed |= leaf.tab_group.tabs != before_tabs;
        }

        let target_pane_id = self
            .resolve_action_pane_id(self.focused_pane)
            .or_else(|| self.leaf_order().into_iter().next())
            .unwrap_or(self.focused_pane);
        if self.find_leaf(target_pane_id).is_none() {
            self.root = workspace_empty_leaf_node(target_pane_id);
            self.set_focused_pane(target_pane_id);
            self.focus_mode_pane = None;
            changed = true;
        }
        let Some(target_leaf) = self.find_leaf_mut(target_pane_id) else {
            return changed;
        };
        for tab_session in tab_sessions {
            let Some(shell_session_id) = workspace_terminal_session_mapping_get(
                &tab_session.key,
                local_workspace_session_mappings,
                remote_attach_sessions,
            ) else {
                continue;
            };
            if assigned_shell_sessions.insert(shell_session_id) {
                target_leaf.tab_group.tabs.push(WorkspaceTab {
                    session_id: shell_session_id,
                });
                changed = true;
            }
        }
        if !target_leaf
            .tab_group
            .tabs
            .iter()
            .any(|tab| tab.session_id == target_leaf.tab_group.active_tab)
        {
            target_leaf.tab_group.active_tab = target_leaf
                .tab_group
                .tabs
                .first()
                .map(|tab| tab.session_id)
                .unwrap_or(TerminalSessionId(0));
            changed = true;
        }
        changed |= self.normalize_workspace_tree();
        changed
    }
}
