//! The chat view's half of the desktop's `launch.rs`, lifted by `build.rs`. The app's half stages a
//! chat in a desktop pane before its session exists; the phone opens only existing sessions.
use super::state::{NativeChatConfig, NativeChatView};
use serde_json::json;

include!(concat!(env!("OUT_DIR"), "/native_chat_launch.rs"));
