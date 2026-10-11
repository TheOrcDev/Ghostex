//! What the host app hands Rust when it starts GPUI.

use std::path::PathBuf;

use serde_json::Value;

/// The start configuration: the app's private files directory plus whatever JSON object the
/// JavaScript side passed to `start(config)`. Only the first start of a process is used; GPUI and
/// its window outlive every Activity and every React Native reload.
#[derive(Clone, Debug)]
pub struct HostConfig {
    /// `Context.getFilesDir()`: where Rust may keep its own files (drafts, caches, SQLite).
    pub files_dir: PathBuf,
    /// The JSON object given to `start(config)`, `{}` when it was not an object.
    pub raw: Value,
}

impl HostConfig {
    pub fn new(files_dir: impl Into<PathBuf>, config_json: &str) -> Self {
        let raw = match serde_json::from_str::<Value>(config_json) {
            Ok(value @ Value::Object(_)) => value,
            Ok(_) => Value::Object(Default::default()),
            Err(error) => {
                log::warn!("start config is not JSON ({error}); using {{}}");
                Value::Object(Default::default())
            }
        };
        Self {
            files_dir: files_dir.into(),
            raw,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.raw.get(key).and_then(Value::as_str)
    }

    pub fn u64(&self, key: &str) -> Option<u64> {
        self.raw.get(key).and_then(Value::as_u64)
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        self.raw.get(key).and_then(Value::as_bool)
    }
}
