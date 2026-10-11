//! The one-time launch of GPUI on the main thread.
//!
//! `runtime::start` may run on any thread and must not touch UIKit, so it only parks the launch
//! here. The first `ghostex_gpui_window` call, which the Swift view makes on the main thread when
//! it first joins a window, builds GPUI's `Application` in gpui-mobile's embedded mode and opens
//! the one window; every later call returns the same window. Like the first surface on Android,
//! the first mounted view is what launches GPUI, so the `ready` event finds a listener.

use std::{
    ffi::c_void,
    sync::{
        Mutex,
        atomic::{AtomicPtr, Ordering},
    },
};

use futures::channel::mpsc::UnboundedReceiver;
use gpui::App;
use serde_json::{Value, json};

use crate::{EventSink, HostConfig};

type Launch = Box<dyn FnOnce(&mut App) + Send>;

static PENDING: Mutex<Option<Launch>> = Mutex::new(None);

/// The `IosWindow` gpui-mobile made, once launched; what every `gpui_ios_*` call takes.
static WINDOW: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Parks the launch for the main thread and applies the platform switches the start config
/// carries. Called once per process by `runtime::start`.
pub(crate) fn prepare(config: HostConfig, commands: UnboundedReceiver<Value>) {
    log::info!(
        "starting GPUI (files dir {}, config keys {:?})",
        config.files_dir.display(),
        config
            .raw
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    );
    // Every GPUI copy (a text selection's Copy, the chat's copy buttons) goes to the host, which
    // writes the system clipboard and tells the user; the same `copy` event as on Android.
    gpui_mobile::ios::set_clipboard_writer(|text| {
        EventSink.emit(json!({"type": "copy", "text": text}));
    });
    // A GPUI panic stops GPUI (its view keeps its last frame) instead of aborting the app, as a
    // panic on Android only ends GPUI's own thread; the screen says so.
    gpui_mobile::ios::panic_guard::set_stop_observer(|message| {
        EventSink.emit(json!({"type": "error", "message": format!("GPUI stopped: {message}")}));
    });
    // The same default as Android (runtime.rs): a touch that stops a fling must not become a tap.
    gpui_mobile::ios::set_fling_guard_enabled(config.bool("flingGuard").unwrap_or(false));
    *PENDING.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(Box::new(move |cx| {
        crate::runtime::launch(cx, config, commands)
    }));
}

/// Main thread only. Launches GPUI on the first call after `prepare` and returns its window, or
/// null before `prepare` (JavaScript has not called `start` yet) or when the launch failed.
pub(super) fn window() -> *mut c_void {
    let existing = WINDOW.load(Ordering::Acquire);
    if !existing.is_null() {
        return existing;
    }
    let Some(launch) = PENDING
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
    else {
        return std::ptr::null_mut();
    };
    let started = std::time::Instant::now();
    gpui_mobile::ios::ffi::gpui_ios_set_embedded();
    gpui_mobile::ios::ffi::set_app_callback(Box::new(move |cx| launch(cx)));
    // The chat's icons and artwork load through the app's asset source.
    gpui_mobile::ios::ffi::run_app_with_assets(ghostex_gpui_mobile_chat::asset_source());
    let window = gpui_mobile::ios::ffi::gpui_ios_get_window();
    if window.is_null() {
        log::error!("GPUI launched without a window");
    } else {
        log::info!("GPUI launched in {:?}", started.elapsed());
    }
    WINDOW.store(window, Ordering::Release);
    window
}
