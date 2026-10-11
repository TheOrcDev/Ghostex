//! The desktop's `model/` is mostly native workspace state; only the plain types the chat files name
//! are here, lifted where the desktop has them in a mixed file (see `extracted-items.txt`).
#[allow(dead_code)]
pub(crate) mod agents_terminal_startup {
    include!(concat!(env!("OUT_DIR"), "/agents_terminal_startup.rs"));
}
pub(crate) use agents_terminal_startup::*;

/// The desktop reaches a remote machine's daemon through an SSH tunnel it owns; the phone reaches
/// each of its computers' daemons through its own forward (`ChatTranscript::set_machine_endpoint`),
/// and a chat on that computer carries it here (`NativeChatConfig::remote`).
#[derive(Clone)]
pub(crate) struct GpuiRemoteGxserverRequestTarget {
    pub(crate) local_port: u16,
    pub(crate) token: String,
}

impl GpuiRemoteGxserverRequestTarget {
    pub(crate) fn endpoint(&self) -> crate::mobile::Endpoint {
        crate::mobile::Endpoint {
            base_url: format!("http://127.0.0.1:{}", self.local_port),
            auth_token: self.token.clone(),
        }
    }
}

/// Only the identity counter of the desktop's chat page state, which the chat view uses for draft
/// ids (`model/session_chat_parking.rs` on the desktop).
pub(crate) struct SessionChatPageState;

impl SessionChatPageState {
    pub(crate) fn next_identity() -> u64 {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
