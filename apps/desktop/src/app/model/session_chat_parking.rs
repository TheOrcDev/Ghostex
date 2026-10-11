// Per-project parking bundle for the Agents session-chat views and the
// companion state keyed by the same project-local shell session ids.

use crate::*;

/*
CDXC:SessionChat 2026-08-26:
Inactive project workspaces keep their live chat views beside their parked shell
models, exactly like `ParkedAgentsTerminalRuntime` keeps their terminal owners,
so a project switch does not rebuild every chat pane on the way back.

Shell session ids are per-`WorkspaceModel` counters and therefore collide across
projects, so every companion map keyed by those ids travels in the same bundle:
leaving one behind would let the incoming project's ids read the outgoing
project's composer readiness, emptiness reports, or one-shot composer messages.
None of this is serialized; it returns only to the exact project that parked it.
*/
#[derive(Default)]
pub(crate) struct ParkedAgentsChatRuntime {
    /// CDXC:SessionChat 2026-09-06 DECISION:
    /// User: a session switched to Terminal must remain there when visiting another project and returning, until switched back manually.
    /// Keep default-view observations with their project so restoring a session does not apply the Chat default again.
    pub(crate) auto_switch_observed_sessions:
        HashMap<TerminalSessionId, GpuiPreferredAgentInterface>,
    pub(crate) page_states: HashMap<TerminalSessionId, SessionChatPageState>,
    pub(crate) native_views:
        HashMap<TerminalSessionId, Entity<crate::app::native_chat::state::NativeChatView>>,
    pub(crate) composer_ready_sessions: HashSet<TerminalSessionId>,
    pub(crate) composer_empty_reports: HashMap<TerminalSessionId, bool>,
    pub(crate) pending_composer_insert: HashMap<TerminalSessionId, String>,
}

/// CDXC:SessionChat 2026-09-05 WHY:
/// Numeric shell session IDs collide across projects, and a chat runtime can answer after its replacement view exists.
/// The binding generation travels through parking with its view.
pub(crate) struct SessionChatPageState {
    pub(crate) generation: u64,
    pub(crate) pending_native_requests: usize,
    pub(crate) account_key: Option<GpuiWorkspaceTerminalSessionKey>,
}

impl SessionChatPageState {
    pub(crate) fn next_identity() -> u64 {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn new() -> Self {
        Self {
            generation: Self::next_identity(),
            pending_native_requests: 0,
            account_key: None,
        }
    }
}

/// Presentation of a pending account change, shared by the sidebar and chat menu.
pub(crate) struct SessionAccountSwitchProgress {
    pub(crate) title: String,
    pub(crate) email: String,
    pub(crate) provider: &'static str,
    pub(crate) indicator: String,
    pub(crate) page_generation: Option<u64>,
}
