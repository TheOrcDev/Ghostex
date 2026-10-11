//! Kotlin to Rust: the `external` functions of `dev.ghostex.gpui.GpuiNative`.
//!
//! Every entry point goes through `EnvUnowned::with_env`, which catches panics at the FFI
//! boundary. Surface and touch entry points log failures and return a default (throwing from a
//! `SurfaceHolder.Callback` would take the app down); `nativeStart` and `nativeCommand` throw a
//! `RuntimeException` so a bad call is visible to the caller.

use gpui_mobile::android::host::{self, Pointer};
use jni::{
    EnvUnowned,
    errors::{LogErrorAndDefault, ThrowRuntimeExAndDefault},
    objects::{JClass, JFloatArray, JIntArray, JObject, JString},
    sys::{jboolean, jfloat, jint, jlong},
};
use ndk::native_window::NativeWindow;

use crate::{HostConfig, runtime};

/// `GpuiNative.nativeStart(activity, filesDir, configJson)`: loads logging, remembers the
/// Activity, installs the event callback and starts GPUI. Returns `true` on the first start of
/// the process, `false` when GPUI was already running (its first configuration stays).
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeStart<'local>(
    mut unowned: EnvUnowned<'local>,
    class: JClass<'local>,
    activity: JObject<'local>,
    files_dir: JString<'local>,
    config_json: JString<'local>,
) -> jboolean {
    super::logging::init();
    unowned
        .with_env(|env| -> jni::errors::Result<bool> {
            if !activity.is_null()
                && let Err(error) = gpui_mobile::android::jni::set_host_activity(env, &activity)
            {
                log::error!("set_host_activity: {error}");
            }
            if runtime::is_started() {
                return Ok(false);
            }
            let vm = env.get_java_vm()?;
            let class = env.new_global_ref(&class)?;
            super::events::install(vm, class);
            super::redraw::install();
            let files_dir = files_dir.try_to_string(env)?;
            let config_json = config_json.try_to_string(env)?;
            Ok(runtime::start(HostConfig::new(files_dir, &config_json)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// `GpuiNative.nativeSetActivity(activity)`: a new Activity instance (recreation) took over.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeSetActivity<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    activity: JObject<'local>,
) {
    unowned
        .with_env(|env| -> jni::errors::Result<()> {
            if let Err(error) = gpui_mobile::android::jni::set_host_activity(env, &activity) {
                log::error!("set_host_activity: {error}");
            }
            Ok(())
        })
        .resolve::<LogErrorAndDefault>()
}

/// `GpuiNative.nativeSurfaceChanged(surface, scale)`: call from `surfaceCreated` and
/// `surfaceChanged` (TextureView: `onSurfaceTextureAvailable` and `onSurfaceTextureSizeChanged`).
/// The same `Surface` again is a resize; a different one is attached in place of the old.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeSurfaceChanged<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    surface: JObject<'local>,
    scale: jfloat,
) {
    unowned
        .with_env(|env| -> jni::errors::Result<()> {
            // SAFETY: `surface` is a live `android.view.Surface` for the duration of this call;
            // `from_surface` takes its own reference on the `ANativeWindow` behind it.
            let window = unsafe {
                NativeWindow::from_surface(env.get_raw().cast(), surface.as_raw().cast())
            };
            match window {
                Some(window) => {
                    log::info!(
                        "surface {}x{} at scale {scale}",
                        window.width(),
                        window.height()
                    );
                    host::surface_created(window, scale);
                }
                None => log::error!("ANativeWindow_fromSurface returned null"),
            }
            Ok(())
        })
        .resolve::<LogErrorAndDefault>()
}

/// `GpuiNative.nativeSurfaceDestroyed()`: blocks until the render thread has let go of the surface
/// (bounded at 2 s by gpui-mobile), so the caller may return from `surfaceDestroyed` afterwards.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeSurfaceDestroyed<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    unowned
        .with_env(|_env| -> jni::errors::Result<()> {
            let started = std::time::Instant::now();
            host::surface_destroyed();
            log::info!("surface released in {:?}", started.elapsed());
            Ok(())
        })
        .resolve::<LogErrorAndDefault>()
}

/// `GpuiNative.nativeSurfaceRedrawNeeded(width, height)`: from `surfaceRedrawNeededAsync`. Rust
/// calls `GpuiNative.onRedrawDone(width, height)` once a frame of that size is on screen.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeSurfaceRedrawNeeded<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    width: jint,
    height: jint,
) {
    unowned
        .with_env(|_env| -> jni::errors::Result<()> {
            super::redraw::requested(width, height);
            Ok(())
        })
        .resolve::<LogErrorAndDefault>()
}

/// `GpuiNative.nativeMotionEvent(action, actionIndex, count, ids, xs, ys, eventTimeMs)`: one whole
/// `MotionEvent`, or one historical sample of a batched `ACTION_MOVE`. `action` is
/// `getActionMasked()`; the arrays hold `count` pointers in index order, positions in physical
/// pixels relative to the surface; `eventTimeMs` is the sample's `getEventTime()` (uptime
/// milliseconds). Returns `false` when GPUI did not take the touch.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeMotionEvent<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    action: jint,
    action_index: jint,
    count: jint,
    ids: JIntArray<'local>,
    xs: JFloatArray<'local>,
    ys: JFloatArray<'local>,
    event_time_ms: jlong,
) -> jboolean {
    let timestamp = instant_from_uptime_ms(event_time_ms);
    unowned
        .with_env(|env| -> jni::errors::Result<bool> {
            let count = count.clamp(0, 16) as usize;
            let mut id_buf = [0i32; 16];
            let mut x_buf = [0f32; 16];
            let mut y_buf = [0f32; 16];
            ids.get_region(env, 0, &mut id_buf[..count])?;
            xs.get_region(env, 0, &mut x_buf[..count])?;
            ys.get_region(env, 0, &mut y_buf[..count])?;
            let mut pointers = [Pointer {
                id: 0,
                x: 0.0,
                y: 0.0,
            }; 16];
            for index in 0..count {
                pointers[index] = Pointer {
                    id: id_buf[index],
                    x: x_buf[index],
                    y: y_buf[index],
                };
            }
            Ok(host::motion_event(
                action as u32,
                action_index.max(0) as usize,
                &pointers[..count],
                timestamp,
            ))
        })
        .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeResumed<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    unowned
        .with_env(|_env| -> jni::errors::Result<()> {
            host::resumed();
            Ok(())
        })
        .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativePaused<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    unowned
        .with_env(|_env| -> jni::errors::Result<()> {
            host::paused();
            Ok(())
        })
        .resolve::<LogErrorAndDefault>()
}

/// `GpuiNative.nativeCommand(json)`: one command object for the root view. Throws on malformed
/// JSON or when GPUI was never started.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_ghostex_gpui_GpuiNative_nativeCommand<'local>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    json: JString<'local>,
) {
    unowned
        .with_env(|env| -> Result<(), CommandError> {
            let json = json.try_to_string(env)?;
            runtime::post_command(&json).map_err(CommandError::Rejected)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[derive(Debug)]
enum CommandError {
    Jni(jni::errors::Error),
    Rejected(anyhow::Error),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Jni(error) => write!(f, "{error}"),
            CommandError::Rejected(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for CommandError {}

impl From<jni::errors::Error> for CommandError {
    fn from(error: jni::errors::Error) -> Self {
        CommandError::Jni(error)
    }
}

/// The `Instant` of an Android uptime timestamp (`MotionEvent.getEventTime()`).
///
/// Both clocks are `CLOCK_MONOTONIC`, so the sample's age is measured against that clock now and
/// taken off `Instant::now()`. A sample from the future or older than a few seconds (a clock this
/// does not understand) gives `None`, and GPUI falls back to the time it processes the touch.
fn instant_from_uptime_ms(event_time_ms: i64) -> Option<std::time::Instant> {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `now` is a valid, writable timespec.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) } != 0 {
        return None;
    }
    let now_ms = now.tv_sec as i64 * 1_000 + now.tv_nsec as i64 / 1_000_000;
    let age_ms = now_ms - event_time_ms;
    if !(0..=5_000).contains(&age_ms) {
        return None;
    }
    std::time::Instant::now().checked_sub(std::time::Duration::from_millis(age_ms as u64))
}
