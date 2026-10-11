//! Messages a client sends on an `/api/events` socket.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Client to server socket message. The server ignores any `type` it does not know.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ClientMessage {
    /// Answered by exactly one `presentationSnapshot` or `presentationSnapshotCurrent`, or by
    /// nothing when the daemon cannot build a snapshot: always pair it with a timeout.
    SubscribePresentation {
        /// Echoed back on the snapshot frame; not used for routing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
        /// The revision the client already applied. Omit on a first connect.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_revision: Option<i64>,
        /// `true` registers this socket as a renderer-command target, which obliges it to answer
        /// every `rendererCommand`. Send it only when ready to handle them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        renderer_commands: Option<bool>,
    },
    /// No acknowledgement frame exists: the reply is the next `sessionChatSnapshot` for the
    /// session, and an unknown session gets no reply at all.
    SubscribeSessionChat {
        project_id: String,
        session_id: String,
        /// Clamped by the server to `0..=10000`, default 300. Raising it respawns the follower
        /// (new epoch); lowering it does nothing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    UnsubscribeSessionChat {
        project_id: String,
        session_id: String,
    },
    RendererCommandResult {
        command_id: String,
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}
