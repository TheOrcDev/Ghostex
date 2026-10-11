//! The phone's `GhostexGpuiApp`: the one method the shared chat code calls on it.
//!
//! A chat view reaches its app through `NativeChatConfig::app`, which the phone leaves `None`: the
//! host is not a GPUI entity, and everything a view hands its app arrives as a `NativeChatEvent`
//! that `mobile/transcript.rs` forwards. The type exists so the shared files type-check.
use crate::*;

pub(crate) struct GhostexGpuiApp;

impl GhostexGpuiApp {
    /// The pane the desktop's keyboard focus is on; a phone surface shows one chat and no panes.
    pub(crate) fn focused_agents_or_companion_shell_session_id(&self) -> Option<TerminalSessionId> {
        None
    }
}
