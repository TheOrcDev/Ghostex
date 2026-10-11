use crate::app::terminal_sync::GpuiTerminalViewerRecipe;
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TerminalSessionId(pub(crate) u64);

/*
CDXC:SessionIdentity 2026-06-22-23:24:
Phase 3 separates process-lifetime Agents terminal runtime identity from durable shell `TerminalSessionId` and pane/body mount slots. Runtime ids bind Ghostty owners to the current app process only; they are not user-facing titles, not logs, not shell-state fields, and restored shell sessions intentionally receive fresh runtime ids.
*/
pub(crate) struct AgentsTerminalRuntimeSessionRegistry {
    pub(crate) runtime_ids_by_shell_session:
        HashMap<TerminalSessionId, AgentsTerminalRuntimeSessionId>,
    pub(crate) next_runtime_session_id: u64,
}

/*
CDXC:Workarea 2026-08-05:
Inactive projects keep direct PTY owners and pending native operations beside their parked shell models.
Daemon-backed viewers retain only attach recipes and recreate the local client when a Terminal pane returns.
Project-local runtime ids, recipes, remaining entities, OSC state, and close-confirm intent travel together so colliding numeric ids cannot restore another project's client; none of this state is serialized.
*/
#[derive(Default)]
pub(crate) struct ParkedAgentsTerminalRuntime {
    pub(crate) runtime_sessions: AgentsTerminalRuntimeSessionRegistry,
    pub(crate) protected_viewer_sessions: HashSet<TerminalSessionId>,
    pub(crate) viewer_recipes: HashMap<TerminalSessionId, GpuiTerminalViewerRecipe>,
    pub(crate) gpui_engine_terminals:
        HashMap<TerminalSessionId, terminal_gpui_engine::GpuiEngineTerminalRecord>,
    /// CDXC:Terminal 2026-09-06 WHY:
    /// Project layout restoration omits zmx names, but reuses live attach clients; losing their identity disables visibility claims while the daemon still ignores ordinary resizes.
    /// Keep the attach names with their process-local owners and restore them before visibility reconciliation.
    pub(crate) zmx_session_names: HashMap<TerminalSessionId, String>,
    pub(crate) runtime_osc_states:
        HashMap<AgentsTerminalRuntimeSessionId, GpuiTerminalRuntimeOscState>,
    pub(crate) gpui_engine_close_confirms: HashSet<AgentsTerminalBodyMountSlotId>,
    /// Viewers that were on screen when the project was left; they skip release while the keep-alive window is open.
    pub(crate) kept_alive_viewer_sessions: HashSet<TerminalSessionId>,
    pub(crate) parked_at: Option<Instant>,
}

impl ParkedAgentsTerminalRuntime {
    pub(crate) fn viewer_kept_alive(
        &self,
        session_id: TerminalSessionId,
        keep: Option<Duration>,
    ) -> bool {
        self.kept_alive_viewer_sessions.contains(&session_id)
            && crate::app::project_keep_alive::project_keep_alive_active(self.parked_at, keep)
    }
}

impl Default for AgentsTerminalRuntimeSessionRegistry {
    fn default() -> Self {
        Self {
            runtime_ids_by_shell_session: HashMap::new(),
            next_runtime_session_id: 1,
        }
    }
}

impl AgentsTerminalRuntimeSessionRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn reconcile_with_workspace(&mut self, workspace: &WorkspaceModel) {
        let shell_session_ids = workspace.terminal_session_ids();
        let current_shell_session_ids = shell_session_ids.iter().copied().collect::<HashSet<_>>();
        self.runtime_ids_by_shell_session
            .retain(|session_id, _| current_shell_session_ids.contains(session_id));

        for session_id in shell_session_ids {
            self.ensure_runtime_session_id(session_id);
        }
    }

    pub(crate) fn runtime_session_id_for_shell_session(
        &self,
        session_id: TerminalSessionId,
    ) -> Option<AgentsTerminalRuntimeSessionId> {
        self.runtime_ids_by_shell_session.get(&session_id).copied()
    }

    pub(crate) fn ensure_runtime_session_id(
        &mut self,
        session_id: TerminalSessionId,
    ) -> AgentsTerminalRuntimeSessionId {
        if let Some(runtime_session_id) = self.runtime_ids_by_shell_session.get(&session_id) {
            return *runtime_session_id;
        }

        let runtime_session_id = self.allocate_runtime_session_id();
        self.runtime_ids_by_shell_session
            .insert(session_id, runtime_session_id);
        runtime_session_id
    }

    pub(crate) fn rotate_runtime_session_id_for_shell_session(
        &mut self,
        session_id: TerminalSessionId,
    ) -> AgentsTerminalRuntimeSessionId {
        /*
        CDXC:Terminal 2026-06-23-18:19:
        Explicit failed-startup retry is a new process-local runtime attempt for the same durable shell session. Rotate only the runtime id so the retry startup candidate, launch plan, and completion intent cannot reuse stale attempt identity while the shell `TerminalSessionId`, tab, title, and persisted state remain unchanged.
        */
        let runtime_session_id = self.allocate_runtime_session_id();
        self.runtime_ids_by_shell_session
            .insert(session_id, runtime_session_id);
        runtime_session_id
    }

    pub(crate) fn allocate_runtime_session_id(&mut self) -> AgentsTerminalRuntimeSessionId {
        let runtime_session_id = AgentsTerminalRuntimeSessionId(self.next_runtime_session_id);
        self.next_runtime_session_id += 1;
        runtime_session_id
    }
}

pub(crate) fn activate_agents_terminal_placeholder_with_runtime_attempt_identity(
    workspace: &mut WorkspaceModel,
    runtime_sessions: &mut AgentsTerminalRuntimeSessionRegistry,
    pane_id: WorkspacePaneId,
    session_id: TerminalSessionId,
) -> bool {
    /*
    CDXC:Terminal 2026-06-23-18:19:
    Placeholder activation may change durable shell presentation, but retry attempt identity is process-local app/runtime state. Detect the explicit `StartupFailed` edge before shell activation, then rotate the runtime id only after that same shell session becomes startup-eligible `Mounting` so wake/materialize/reattach placeholders keep their existing runtime identity and cannot enter a retry startup attempt.

    CDXC:Terminal 2026-06-23-19:26:
    Restored-unmounted materialization now enters the startup pipeline, but it is not a retry. Keep the process-local runtime id already associated with the durable shell session; sleeping wake and popped-out reattach remain blocked from startup maps and use the separate slice 236 parked-owner contract.
    */
    let retry_activation = workspace.session(session_id).is_some_and(|session| {
        session.presentation_state == TerminalSessionPresentationState::StartupFailed
    });
    let model_changed = workspace.activate_terminal_placeholder_session(pane_id, session_id);

    if retry_activation
        && workspace
            .session(session_id)
            .is_some_and(TerminalSession::can_enter_startup_pipeline)
    {
        runtime_sessions.rotate_runtime_session_id_for_shell_session(session_id);
    }

    model_changed
}
