//! Per-machine connection state, as reported by the host's socket client.
//!
//! The core does not connect to anything. The client that owns a machine's event socket tells the
//! core what happened, and the store keeps it next to the machine's rows so a UI can draw a machine
//! as connecting, live, or stale without asking the client.

use serde::{Deserialize, Serialize};

/// Where a machine's event stream stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ConnectionPhase {
    /// Nothing has been tried yet.
    #[default]
    Idle,
    /// A socket is being opened or a subscribe is waiting for its answer.
    Connecting,
    /// The stream is acknowledged: a snapshot or a snapshot-current answer arrived on it.
    Live,
    /// The stream dropped while rows are held. The rows are the last known state, not the truth:
    /// draw them as stale and do not act on them.
    Stale,
    /// The stream is down and nothing is held.
    Disconnected,
}

/// Connection state of one machine.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ConnectionState {
    pub phase: ConnectionPhase,
    /// The most recent failure, kept while reconnecting so the UI can say why. Cleared when the
    /// stream goes live.
    pub last_error: Option<String>,
    /// Reconnect attempt the client is on; 0 while live or idle.
    pub attempt: u32,
    /// Host time of the last phase change.
    pub changed_at_ms: u64,
}

/// What the socket client reports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ConnectionUpdate {
    /// Opening a socket or waiting for the subscribe answer.
    Connecting { attempt: u32 },
    /// The stream is acknowledged. The core also sets this itself when a stream snapshot or a
    /// snapshot-current answer is applied.
    Live,
    /// The socket closed or failed. A close means frames may have been missed: the client must
    /// resubscribe, and until then the held rows are stale.
    Lost { error: Option<String> },
}
