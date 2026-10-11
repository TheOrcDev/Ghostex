//! iOS glue: the C ABI the Swift `GpuiView` Expo module calls (`include/ghostex_gpui.h`), and the
//! main-thread launch of GPUI.
//!
//! On iOS GPUI runs on the main thread, UIKit's, not on a thread of its own as on Android: its
//! window is a `UIViewController` whose view is a `CAMetalLayer`-backed `UIView`
//! (`.dependencies/gpui-mobile/src/ios`, embedded mode), which the Swift view adds as a child of
//! the React Native view and drives with a `CADisplayLink` through gpui-mobile's own `gpui_ios_*`
//! calls. This module only adds what the host needs on top: start, the event callback, commands
//! and the one-time launch.
//!
//! Threads: `ghostex_gpui_start`, `ghostex_gpui_command` and `ghostex_gpui_set_event_callback` may
//! run on any thread (Expo calls synchronous functions on the JavaScript thread); they never touch
//! GPUI. `ghostex_gpui_window` must run on the main thread, as must every `gpui_ios_*` call.
//! Events are emitted from the main thread, where GPUI runs.

mod entry;
mod launch;
mod logging;

pub(crate) use launch::prepare;
