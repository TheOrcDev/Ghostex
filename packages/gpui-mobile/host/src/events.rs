//! Rust to host events.
//!
//! Every event is a JSON object with a `type` field. On Android the platform layer installs an
//! emitter that calls the Kotlin `GpuiNative.onRustEvent(String)`, which posts it to the UI thread
//! and dispatches it as the React Native view's `onGpuiEvent`. Events are fire-and-forget: the
//! host may not be listening (no view mounted, a reload in progress) and nothing waits for them.

use std::sync::OnceLock;

use serde_json::Value;

type Emitter = Box<dyn Fn(&str) + Send + Sync>;

static EMITTER: OnceLock<Emitter> = OnceLock::new();

/// Installs the platform's delivery function. Only the first call wins.
pub fn set_emitter(emitter: impl Fn(&str) + Send + Sync + 'static) {
    if EMITTER.set(Box::new(emitter)).is_err() {
        log::warn!("event emitter already installed");
    }
}

/// A cheap handle root views keep to send events to the host.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventSink;

impl EventSink {
    /// Sends `event` (an object with a `type`) to the host. Callable from any thread.
    pub fn emit(&self, event: Value) {
        emit_json(&event.to_string());
    }
}

fn emit_json(json: &str) {
    match EMITTER.get() {
        Some(emitter) => emitter(json),
        None => log::debug!("event dropped, no emitter: {json}"),
    }
}
