//! Android glue: JNI entry points for the Kotlin `dev.ghostex.gpui.GpuiNative` object and the
//! callback that carries events back to it.
//!
//! Threads: every `native*` function is called on the Android UI thread (or, for `nativeCommand`,
//! the JS thread). None of them touch GPUI; they queue work for gpui-mobile's `gpui-main` thread
//! (`gpui_mobile::android::host`) or for the command channel ([`crate::runtime`]). Events are sent
//! from `gpui-main`, which attaches itself to the JVM once and stays attached.

mod entry;
mod events;
mod logging;
mod redraw;

/// gpui-mobile links `android-activity`'s NativeActivity glue, whose `ANativeActivity_onCreate`
/// calls an `android_main` the app must define, or the library fails to `dlopen`. This app never
/// launches a `NativeActivity` (React Native owns the Activity), so it is never called.
#[unsafe(no_mangle)]
fn android_main(_app: android_activity::AndroidApp) {
    log::error!(
        "android_main called: this library is hosted by a React Native Activity, not a NativeActivity"
    );
}
