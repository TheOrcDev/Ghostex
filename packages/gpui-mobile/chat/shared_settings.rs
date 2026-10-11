//! The shared settings object as the chat code reads it. The desktop's module owns the settings
//! FILE (watching, migrating, writing); the phone only holds the object its host hands it
//! (`ChatInit::settings`, `mobile::install_settings`), behind the same names the shared files call.
//!
//! One process-wide value rather than the web build's thread-local, because the chat host reads it
//! from its own thread (`gx_chat/boot.rs`, `gx_chat/diagnostics.rs`).
mod appearance;
#[allow(unused_imports)]
pub use appearance::*;

use std::sync::{Arc, RwLock};

use serde_json::{Map, Value};

#[derive(Clone, Debug, Default)]
pub struct SharedSidebarSettingsSnapshot {
    revision: u64,
    object: Arc<Map<String, Value>>,
}

#[allow(dead_code)]
impl SharedSidebarSettingsSnapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn content_hash(&self) -> u64 {
        self.revision
    }

    pub fn object(&self) -> &Map<String, Value> {
        &self.object
    }

    pub fn debugging_mode(&self) -> bool {
        self.object.get("debuggingMode").and_then(Value::as_bool) == Some(true)
    }
}

static CURRENT: RwLock<Option<SharedSidebarSettingsSnapshot>> = RwLock::new(None);

pub fn shared_sidebar_settings_snapshot() -> SharedSidebarSettingsSnapshot {
    CURRENT
        .read()
        .ok()
        .and_then(|current| current.clone())
        .unwrap_or_default()
}

/// Installs a settings object (the desktop's `settings.json` shape).
pub fn install(object: Map<String, Value>) {
    if let Ok(mut current) = CURRENT.write() {
        let revision = current.as_ref().map_or(0, |current| current.revision) + 1;
        *current = Some(SharedSidebarSettingsSnapshot {
            revision,
            object: Arc::new(object),
        });
    }
}
