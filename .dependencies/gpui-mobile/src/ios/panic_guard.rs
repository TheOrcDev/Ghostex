//! Ghostex patch: a GPUI panic stops GPUI instead of the whole app.
//!
//! On iOS GPUI runs on the main thread, and UIKit enters it only through `extern "C"`
//! functions (the display link's `gpui_ios_request_frame`, the touch methods of GPUI's view,
//! layout, lifecycle, and the GCD trampoline that runs GPUI's tasks). A panic unwinding out of
//! any of them aborts the process, taking the host app with it. The Android host, where GPUI
//! has a thread of its own, only loses that thread (GHOSTEX.md, patch 13).
//!
//! Each main-thread entry runs through [`contain`]. The first panic marks GPUI as stopped: its
//! state may be half-updated, so no entry calls into it again (frames report no demand, touches
//! and tasks are dropped), the host observer is told once, and the host app keeps running with
//! the last frame on screen. A panic in a background task is caught and logged without stopping
//! GPUI; only that task is lost.

use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

static STOPPED: AtomicBool = AtomicBool::new(false);

type StopObserver = Box<dyn Fn(&str) + Send + Sync>;

static STOP_OBSERVER: OnceLock<StopObserver> = OnceLock::new();

/// `observer(message)` runs once, on the main thread, when a panic stops GPUI. Only the first
/// observer is kept.
pub fn set_stop_observer(observer: impl Fn(&str) + Send + Sync + 'static) {
    let _ = STOP_OBSERVER.set(Box::new(observer));
}

/// Whether a panic has stopped GPUI.
pub fn is_stopped() -> bool {
    STOPPED.load(Ordering::Acquire)
}

/// Runs `f`, a main-thread entry into GPUI named `what`. Returns `stopped` without running it
/// once GPUI is stopped, or when it panics (which stops GPUI).
pub(crate) fn contain<R>(what: &str, stopped: R, f: impl FnOnce() -> R) -> R {
    if is_stopped() {
        return stopped;
    }
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => {
            STOPPED.store(true, Ordering::Release);
            let message = format!("{what} panicked: {}", panic_text(&*payload));
            log::error!("GPUI iOS: {message}; GPUI is stopped, the app keeps running");
            if let Some(observer) = STOP_OBSERVER.get() {
                observer(&message);
            }
            stopped
        }
    }
}

/// Runs a background task, logging a panic instead of aborting the process.
pub(crate) fn contain_background(f: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(f)) {
        log::error!(
            "GPUI iOS: a background task panicked: {}",
            panic_text(&*payload)
        );
    }
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| text.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_string())
}
