// The C ABI of libghostex_gpui_mobile.a (packages/gpui-mobile/host) on iOS: a GPUI surface
// embedded in a React Native view. The Swift side is the `GpuiView` Expo module
// (apps/gpui-mobile-poc/app/modules/gpui-view/ios); `scripts/build-ios.sh` packages this header
// and its module map (`import GhostexGpuiMobile`) with the library in an XCFramework.
//
// Threads: the ghostex_gpui_start / _command / _set_event_callback functions may be called on any
// thread. Everything else, including every gpui_ios_* function, is main thread only: GPUI runs on
// UIKit's main thread.
//
// The protocol carried by commands and events is the same JSON as on Android
// (host/src/chat_root.rs, app/modules/gpui-view/src/index.tsx).

#ifndef GHOSTEX_GPUI_H
#define GHOSTEX_GPUI_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ---- Host (host/src/ios/entry.rs) ----------------------------------------------------------

/// One event: `json` is a NUL-terminated JSON object with a `type`, valid only during the call.
/// Called on the main thread for everything GPUI emits; a handler must not assume it.
typedef void (*ghostex_gpui_event_fn)(const char *json, void *context);

/// Installs the event handler. Only the first call has an effect; `context` must stay valid for
/// the life of the process.
void ghostex_gpui_set_event_callback(ghostex_gpui_event_fn callback, void *context);

/// Starts the host once per process: logging, the command channel, the launch configuration.
/// `files_dir` is a private writable directory (the chat's client storage lives in `chat/`);
/// `config_json` is the object JavaScript passed to `start(config)`. GPUI itself launches at the
/// first ghostex_gpui_window(). Returns false when already started (the first config stays).
bool ghostex_gpui_start(const char *files_dir, const char *config_json);

/// The GPUI window, launching GPUI on the first call. NULL before ghostex_gpui_start(), off the
/// main thread, or when the launch failed. Pass it to the gpui_ios_* functions below.
void *ghostex_gpui_window(void);

/// Queues one JSON command (an object with a `type`) for the root view. Returns NULL when queued,
/// or an error message to free with ghostex_gpui_string_free().
char *ghostex_gpui_command(const char *json);

void ghostex_gpui_string_free(char *text);

// ---- gpui-mobile (.dependencies/gpui-mobile/src/ios/ffi.rs) --------------------------------

/// The window's UIViewController (unretained), for child view controller containment. Its view is
/// the CAMetalLayer-backed view GPUI draws into and receives touches on.
void *gpui_ios_view_controller(void *window);

/// Call after changing the controller view's frame: GPUI resizes its drawable and relayouts.
void gpui_ios_layout_view(void *window);

/// Gives GPUI a frame (it draws only if something changed). Returns whether it wants another.
bool gpui_ios_request_frame(void *window);

/// `waker(context)` is called on the main thread when GPUI wants frames again after
/// gpui_ios_request_frame() returned false. NULL clears it.
void gpui_ios_set_frame_waker(void *window, void (*waker)(void *context), void *context);

/// Application lifecycle, from the matching UIApplication notifications. The pointer is unused.
void gpui_ios_will_enter_foreground(void *app);
void gpui_ios_did_become_active(void *app);
void gpui_ios_will_resign_active(void *app);
void gpui_ios_did_enter_background(void *app);

#ifdef __cplusplus
}
#endif

#endif // GHOSTEX_GPUI_H
