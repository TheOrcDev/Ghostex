//! Logcat output for Rust (`adb logcat -s GhostexGpui`).

use std::sync::Once;

/// Routes `log` to logcat under one tag and panics to `log::error!`. Must run before gpui-mobile
/// builds its platform: its own logger init is a no-op once a logger exists, so all GPUI,
/// gpui-mobile and wgpu output lands under this tag and filter.
pub(super) fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("GhostexGpui")
                .with_max_level(log::LevelFilter::Info)
                .with_filter(
                    android_logger::FilterBuilder::new()
                        .parse("info,wgpu_core=warn,wgpu_hal=warn,naga=warn")
                        .build(),
                ),
        );
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
