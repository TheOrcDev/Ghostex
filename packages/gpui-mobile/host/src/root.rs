//! The swap point: what the embedded window shows.
//!
//! [`build_root`] runs once, on the GPUI thread, when the first surface arrives. It returns the
//! view to draw and the handler for commands the host sends to it: the chat transcript
//! ([`crate::chat_root`]), or with `"content": "demo"` in the start config the synthetic list the
//! embedding was first proven with ([`crate::demo`]), kept to compare scrolling against.
//!
//! Commands reach `on_command` as parsed JSON objects with a `type`; host-level ones (`ping`) are
//! answered by [`crate::host_view`] before they get here. Return `false` for a command the content
//! does not know, and the host reports it back as an `error` event.

use gpui::{AnyView, App, Window};
use serde_json::Value;

use crate::{EventSink, HostConfig};

/// A command handler: `(command, window, cx) -> handled`.
pub type CommandHandler = Box<dyn FnMut(&Value, &mut Window, &mut App) -> bool>;

pub struct RootContent {
    /// Drawn filling the whole surface.
    pub view: AnyView,
    pub on_command: CommandHandler,
}

pub fn build_root(
    window: &mut Window,
    cx: &mut App,
    config: &HostConfig,
    events: EventSink,
) -> RootContent {
    match config.str("content") {
        Some("demo") => crate::demo::build(window, cx, config, events),
        _ => crate::chat_root::build(window, cx, config, events),
    }
}
