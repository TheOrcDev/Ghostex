//! Rust logging on iOS: one line per record, tagged `GhostexGpui`, to standard error and to
//! `<files>/logs/gpui.log` in the app's container (started over at every launch).
//!
//! `xcrun simctl launch --stderr=<file>` (what `scripts/run-ios.sh` does) and Xcode's console
//! capture standard error; the file keeps the log of an app launched any other way (from the home
//! screen, by a UI test). Panics are logged with a backtrace before they unwind.

use std::io::Write as _;
use std::path::Path;
use std::sync::{Mutex, Once};

use log::{Level, LevelFilter, Log, Metadata, Record};

struct HostLogger;

static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Crates whose info output is noise for this host (wgpu's per-frame and per-resource lines).
const QUIET_TARGETS: &[&str] = &["wgpu_core", "wgpu_hal", "naga"];

impl Log for HostLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let quiet = QUIET_TARGETS
            .iter()
            .any(|target| metadata.target().starts_with(target));
        metadata.level() <= if quiet { Level::Warn } else { Level::Info }
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "GhostexGpui {:<5} {}: {}\n",
            record.level(),
            record.target(),
            record.args()
        );
        // One write per record, so lines from different threads do not interleave.
        let _ = std::io::stderr().lock().write_all(line.as_bytes());
        if let Ok(mut file) = FILE.lock()
            && let Some(file) = file.as_mut()
        {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// Also writes the log to `path`, replacing what the previous launch left there.
pub(super) fn log_to_file(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::File::create(path) {
        Ok(file) => {
            if let Ok(mut slot) = FILE.lock() {
                *slot = Some(file);
            }
            log::info!("logging to {}", path.display());
        }
        Err(error) => log::warn!("cannot log to {}: {error}", path.display()),
    }
}

pub(super) fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        static LOGGER: HostLogger = HostLogger;
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(LevelFilter::Info);
        }
        std::panic::set_hook(Box::new(|info| {
            let payload = info
                .payload()
                .downcast_ref::<&str>()
                .map(|text| text.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".to_string());
            let location = info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()))
                .unwrap_or_default();
            let thread = std::thread::current();
            log::error!(
                "PANIC on thread {:?} at {location}: {payload}\n{}",
                thread.name().unwrap_or("?"),
                std::backtrace::Backtrace::force_capture()
            );
        }));
        log::info!("ghostex-gpui-mobile {} loaded", env!("CARGO_PKG_VERSION"));
    });
}
