//! The gxserver contract the chat brain consumes, reusing `packages/gx-protocol` for the frames.

mod frames;
mod rpc;

pub use crate::wire::frames::{
    ChatAppendedFrame, ChatBlock, ChatFrame, ChatFrameBase, ChatMessage, ChatRole, ChatSideState,
    ChatSnapshotFrame, ChatStateFrame, ChatStatus, ClientMessage, ReadSessionChatResult,
    ServerEvent, StreamPosition, TurnLifecycle, CHAT_FRAME_TYPES,
    GXSERVER_EVENTS_STREAM_SESSION_CHAT, GXSERVER_PROTOCOL_VERSION,
};
pub use crate::wire::rpc::{ChatRpcMethod, RpcErrorCode, RpcOutcome};
