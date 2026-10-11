//! Swift to Rust: the `ghostex_gpui_*` functions declared in `include/ghostex_gpui.h`.
//!
//! Every entry point catches panics at the boundary (a panic unwinding into Swift would abort the
//! app) and reports them as a failure. The frame, layout, touch and lifecycle calls are
//! gpui-mobile's own `gpui_ios_*` functions, which the header re-declares.

use std::{
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::OnceLock,
};

use crate::{HostConfig, runtime};

/// `callback(json, context)`: one event, a NUL-terminated JSON object with a `type`, valid only
/// for the duration of the call.
pub type EventCallback = unsafe extern "C" fn(json: *const c_char, context: *mut c_void);

struct Listener {
    callback: EventCallback,
    context: *mut c_void,
}

// SAFETY: the host promises the callback may be called from any thread with this context (the
// Swift side hops to the main thread itself) and keeps the context alive for the process.
unsafe impl Send for Listener {}
unsafe impl Sync for Listener {}

static LISTENER: OnceLock<Listener> = OnceLock::new();

unsafe extern "C" {
    /// libSystem: non-zero on the process's main thread.
    fn pthread_main_np() -> i32;
}

/// Installs the function events are delivered to. Only the first call has an effect. Any thread.
#[unsafe(no_mangle)]
pub extern "C" fn ghostex_gpui_set_event_callback(
    callback: Option<EventCallback>,
    context: *mut c_void,
) {
    super::logging::init();
    let Some(callback) = callback else {
        return;
    };
    if LISTENER.set(Listener { callback, context }).is_err() {
        log::warn!("event callback already installed");
        return;
    }
    crate::events::set_emitter(|json| {
        let Some(listener) = LISTENER.get() else {
            return;
        };
        match CString::new(json) {
            // SAFETY: the callback and context are the host's, valid for the process; the string
            // lives until the call returns.
            Ok(text) => unsafe { (listener.callback)(text.as_ptr(), listener.context) },
            Err(_) => log::error!("event with an interior NUL dropped"),
        }
    });
}

/// Starts the host once per process: logging, the command channel and the launch configuration.
/// GPUI itself launches on the main thread at the first [`ghostex_gpui_window`]. Returns `true` on
/// the first start, `false` when it was already started (a reloaded JavaScript bundle; the first
/// configuration stays) or the arguments were unusable. Any thread.
///
/// # Safety
/// `files_dir` and `config_json` must be null or NUL-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ghostex_gpui_start(
    files_dir: *const c_char,
    config_json: *const c_char,
) -> bool {
    super::logging::init();
    // SAFETY: forwarded from the caller's contract.
    let (Some(files_dir), config_json) = (unsafe { text(files_dir) }, unsafe { text(config_json) })
    else {
        log::error!("ghostex_gpui_start: no files directory");
        return false;
    };
    if !runtime::is_started() {
        super::logging::log_to_file(&std::path::Path::new(&files_dir).join("logs/gpui.log"));
    }
    let config = HostConfig::new(files_dir, config_json.as_deref().unwrap_or("{}"));
    catch_unwind(AssertUnwindSafe(|| runtime::start(config))).unwrap_or_else(|_| {
        log::error!("ghostex_gpui_start panicked");
        false
    })
}

/// The GPUI window (gpui-mobile's `IosWindow`, the pointer every `gpui_ios_*` call takes),
/// launching GPUI on the first call. Null before [`ghostex_gpui_start`], off the main thread, or
/// when the launch failed. Main thread only.
#[unsafe(no_mangle)]
pub extern "C" fn ghostex_gpui_window() -> *mut c_void {
    // SAFETY: a plain libSystem query.
    if unsafe { pthread_main_np() } == 0 {
        log::error!("ghostex_gpui_window called off the main thread");
        return std::ptr::null_mut();
    }
    catch_unwind(super::launch::window).unwrap_or_else(|_| {
        log::error!("launching GPUI panicked");
        std::ptr::null_mut()
    })
}

/// Queues one JSON command (an object with a `type`) for the root view. Returns null when queued,
/// or an error message the caller frees with [`ghostex_gpui_string_free`]. Any thread.
///
/// # Safety
/// `json` must be null or a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ghostex_gpui_command(json: *const c_char) -> *mut c_char {
    // SAFETY: forwarded from the caller's contract.
    let Some(json) = (unsafe { text(json) }) else {
        return error_string("a command must be a UTF-8 JSON string");
    };
    match catch_unwind(AssertUnwindSafe(|| runtime::post_command(&json))) {
        Ok(Ok(())) => std::ptr::null_mut(),
        Ok(Err(error)) => error_string(&format!("{error:#}")),
        Err(_) => error_string("posting the command panicked"),
    }
}

/// Frees a string returned by this library.
///
/// # Safety
/// `text` must be null or a pointer this library returned, freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ghostex_gpui_string_free(text: *mut c_char) {
    if !text.is_null() {
        // SAFETY: made by `CString::into_raw` in `error_string`.
        drop(unsafe { CString::from_raw(text) });
    }
}

/// # Safety
/// `pointer` must be null or a NUL-terminated string.
unsafe fn text(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: the caller's contract.
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .ok()
        .map(str::to_string)
}

fn error_string(message: &str) -> *mut c_char {
    CString::new(message.replace('\0', " "))
        .unwrap_or_default()
        .into_raw()
}
