//! Rust to Kotlin: static methods of `GpuiNative`, called from `gpui-main`.

use std::sync::OnceLock;

use jni::{
    JavaVM,
    objects::{JClass, JValue},
    refs::Global,
};

/// The `GpuiNative` class, pinned at start. A class looked up from `gpui-main` would go through the
/// system class loader, which cannot see app classes.
static CALLBACK_CLASS: OnceLock<(JavaVM, Global<JClass<'static>>)> = OnceLock::new();

pub(super) fn install(vm: JavaVM, class: Global<JClass<'static>>) {
    if CALLBACK_CLASS.set((vm, class)).is_err() {
        return;
    }
    crate::events::set_emitter(|json| {
        with_class("onRustEvent", |env, class| {
            let text = env.new_string(json)?;
            env.call_static_method(
                class,
                jni::jni_str!("onRustEvent"),
                jni::jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&text)],
            )?;
            Ok(())
        });
    });
}

/// `GpuiNative.onRedrawDone(width, height)`: a frame of that size is on screen.
pub(super) fn redraw_done(width: i32, height: i32) {
    with_class("onRedrawDone", |env, class| {
        env.call_static_method(
            class,
            jni::jni_str!("onRedrawDone"),
            jni::jni_sig!("(II)V"),
            &[JValue::Int(width), JValue::Int(height)],
        )?;
        Ok(())
    });
}

fn with_class(
    what: &str,
    call: impl FnOnce(&mut jni::Env, &Global<JClass<'static>>) -> jni::errors::Result<()>,
) {
    let Some((vm, class)) = CALLBACK_CLASS.get() else {
        return;
    };
    // Attaches the calling thread once; later calls only read thread-local state.
    if let Err(error) = vm.attach_current_thread(|env| call(env, class)) {
        log::error!("GpuiNative.{what} failed: {error}");
    }
}
