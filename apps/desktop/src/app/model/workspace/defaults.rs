use crate::*;

#[derive(Clone)]
pub(crate) struct WorkspaceModel {
    pub(crate) terminal_sessions: Vec<TerminalSession>,
    pub(crate) root: WorkspaceNode,
    pub(crate) focused_pane: WorkspacePaneId,
    /// Most recently focused panes, oldest first. Runtime-only; seeded with the restored focused pane.
    pub(crate) pane_focus_history: Vec<WorkspacePaneId>,
    pub(crate) focus_mode_pane: Option<WorkspacePaneId>,
    pub(crate) next_pane_id: u64,
    pub(crate) next_split_id: u64,
    pub(crate) next_session_id: u64,
}

impl WorkspaceModel {
    pub(crate) fn empty_default() -> Self {
        let pane_id = WorkspacePaneId(1);
        Self {
            terminal_sessions: Vec::new(),
            root: workspace_empty_leaf_node(pane_id),
            focused_pane: pane_id,
            pane_focus_history: vec![pane_id],
            focus_mode_pane: None,
            next_pane_id: 2,
            next_split_id: 1,
            next_session_id: 1,
        }
    }

    #[allow(dead_code)] // no caller: the CDXC:Workarea sample workspace is not built at startup any more
    pub(crate) fn first_slice_default() -> Self {
        /*
        CDXC:Workarea 2026-06-22-05:23:
        Agents terminal tabs need explicit user-facing presentation states before the runtime lifecycle exists. Running, sleeping, mounting, failed startup, restored/unmounted, and popped-out placeholder sessions stay in the same tab/split layout tree so tab selection can show the correct body state without deleting, waking, or hiding sessions.

        CDXC:SessionStatus 2026-06-22-23:52:
        The default GPUI Agents workspace must visibly exercise non-idle semantic running-tab indicators while terminal bodies remain black placeholders: working, attention, and Delayed Send. Idle running tabs render without a status dot; lifecycle placeholder samples remain separate from running activity.
        */
        let terminal_sessions = vec![
            TerminalSession::placeholder(
                TerminalSessionId(1),
                "Agent".to_string(),
                TerminalSessionPresentationState::Running,
            )
            .with_agent_icon(Some("codex")),
            TerminalSession::placeholder(
                TerminalSessionId(2),
                "Build".to_string(),
                TerminalSessionPresentationState::Running,
            )
            .with_agent_icon(Some("codex"))
            .with_activity(AgentTerminalActivity::Working),
            TerminalSession::placeholder(
                TerminalSessionId(3),
                "Review".to_string(),
                TerminalSessionPresentationState::Running,
            )
            .with_agent_icon(Some("claude"))
            .with_activity(AgentTerminalActivity::Attention),
            TerminalSession::placeholder(
                TerminalSessionId(4),
                "Delayed Send".to_string(),
                TerminalSessionPresentationState::Running,
            )
            .with_agent_icon(Some("codex"))
            .with_activity(AgentTerminalActivity::Working)
            .with_delayed_send_active(true),
            TerminalSession::placeholder(
                TerminalSessionId(5),
                "Sleeping".to_string(),
                TerminalSessionPresentationState::Sleeping,
            ),
            TerminalSession::placeholder(
                TerminalSessionId(6),
                "Shell".to_string(),
                TerminalSessionPresentationState::Mounting,
            ),
            TerminalSession::placeholder(
                TerminalSessionId(7),
                "Restored".to_string(),
                TerminalSessionPresentationState::RestoredUnmounted,
            ),
            TerminalSession::placeholder(
                TerminalSessionId(8),
                "Detached".to_string(),
                TerminalSessionPresentationState::PoppedOutPlaceholder,
            ),
        ];
        let pane_id = WorkspacePaneId(1);
        let active_tab = terminal_sessions[0].id;
        let tabs = terminal_sessions
            .iter()
            .map(|session| WorkspaceTab {
                session_id: session.id,
            })
            .collect();

        /*
        CDXC:Workarea 2026-06-22-05:11:
        Agents mode needs a GPUI-owned terminal workspace model before libghostty is mounted. Seed multiple terminal sessions into one tab group so ordinary sessions preserve tab order, active tab, and pane ownership instead of creating implicit split panes.
        */
        Self {
            terminal_sessions,
            root: WorkspaceNode::Leaf(WorkspaceLeaf {
                pane_id,
                tab_group: WorkspaceTabGroup { tabs, active_tab },
            }),
            focused_pane: pane_id,
            pane_focus_history: vec![pane_id],
            focus_mode_pane: None,
            next_pane_id: 2,
            next_split_id: 1,
            next_session_id: 9,
        }
    }
}
