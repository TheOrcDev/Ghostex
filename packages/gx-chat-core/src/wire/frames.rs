//! The gxserver session-chat frames the brain folds.
//!
//! The frame bodies live in `packages/gx-protocol`, which is the typed description of the gxserver
//! wire contract for every Rust client. Nothing is redefined here; this module only names the four
//! frames chat accepts and the two messages it sends, so a reader sees the chat subset in one
//! place.

pub use ghostex_gx_protocol::{
    ChatAppendedFrame, ChatBlock, ChatFrameBase, ChatMessage, ChatRole, ChatSideState,
    ChatSnapshotFrame, ChatStateFrame, ChatStatus, ClientMessage, ReadSessionChatResult,
    ServerEvent, TurnLifecycle, GXSERVER_EVENTS_STREAM_SESSION_CHAT, GXSERVER_PROTOCOL_VERSION,
};

/// The four `type` values the chat socket accepts.
///
/// `apps/desktop/sidebar/session-chat-runtime/socket.ts` drops anything else before it reaches the
/// fold, so this list is the whole inbound surface.
pub const CHAT_FRAME_TYPES: [&str; 4] = [
    "sessionChatSnapshot",
    "sessionChatReplaced",
    "sessionChatAppended",
    "sessionChatState",
];

/// One accepted frame, already parsed.
///
/// `Replaced` carries the same body as `Snapshot` but means "this transcript was rewritten", which
/// the fold treats differently from a first read.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatFrame {
    Snapshot(Box<ChatSnapshotFrame>),
    Replaced(Box<ChatSnapshotFrame>),
    Appended(Box<ChatAppendedFrame>),
    State(Box<ChatStateFrame>),
}

impl ChatFrame {
    /// The wire `type` of this frame.
    pub fn wire_type(&self) -> &'static str {
        match self {
            Self::Snapshot(_) => "sessionChatSnapshot",
            Self::Replaced(_) => "sessionChatReplaced",
            Self::Appended(_) => "sessionChatAppended",
            Self::State(_) => "sessionChatState",
        }
    }

    /// The frame's stream position, which the fold uses to drop duplicates and spot gaps.
    pub fn position(&self) -> StreamPosition {
        let base = match self {
            Self::Snapshot(frame) | Self::Replaced(frame) => &frame.base,
            Self::Appended(frame) => &frame.base,
            Self::State(frame) => &frame.base,
        };
        StreamPosition {
            server_id: base.server_id.clone(),
            epoch: base.epoch,
            seq: base.seq,
        }
    }
}

/// Where a frame sits in the stream. A frame at or behind the accepted position is dropped; a
/// frame more than one ahead is a gap and asks for a resync read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamPosition {
    pub server_id: String,
    pub epoch: i64,
    pub seq: i64,
}

impl StreamPosition {
    /// Whether this position is strictly ahead of `reference` on the same server.
    pub fn is_ahead_of(&self, reference: &Self) -> bool {
        self.epoch > reference.epoch || (self.epoch == reference.epoch && self.seq > reference.seq)
    }
}
