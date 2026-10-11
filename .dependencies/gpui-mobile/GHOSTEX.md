# gpui-mobile in Ghostex

Vendored copy of Longbridge's fork of gpui-mobile, the GPUI platform backend for Android and iOS.
Ghostex uses its Android host-driven mode (`src/android/host.rs`): a React Native view owns the
`SurfaceView`/`TextureView`, and GPUI renders into it on a process-lived `gpui-main` thread. On iOS
it uses the embedded mode (`src/ios/ffi.rs`, `gpui_ios_set_embedded`): GPUI's view controller is a
child of the React Native view, driven by the host's `CADisplayLink` on the main thread. The
consumer is `packages/gpui-mobile/host` (crate `ghostex-gpui-mobile`).

## Source

- Repository: https://github.com/longbridge/gpui-mobile
- Commit: `f379bc8` ("feat(ios): deregister IosWindow from the FFI window list on drop (#19)")
- Copied: `src/`, `Cargo.toml`, `LICENSE-APACHE`, `LICENSE-GPL`, `LICENSE-AGPL`, `README.md`. Not
  copied: `example/`, `screenshots/`, `.github/`, `.cargo/`, `PACKAGES-TO-IMPL.md`, `TODO.md`.
- Licence: the crate is GPL-3.0-or-later OR AGPL-3.0-or-later OR Apache-2.0; Ghostex takes it under
  **Apache-2.0** (the same licence as GPUI). The licence files are kept as shipped.

## Local patches

Every change against upstream `f379bc8`, so a re-sync can re-apply them. Diff against a fresh
clone to see them exactly (`diff -ru <clone>/src src`).

1. **`Cargo.toml`: GPUI from the Zed fork.** `gpui` and `gpui_wgpu` point at
   `git = "https://github.com/zed-industries/zed"` instead of the `gpui-pre` / `gpui-pre-wgpu`
   crates.io snapshots; the consuming workspace's `[patch."https://github.com/zed-industries/zed"]`
   redirects them to `.dependencies/zed`, so the platform and the app share one GPUI.
2. **`Cargo.toml`: no default features.** Upstream enables every `packages` feature (camera,
   contacts, location, webview, ...); none are needed to host a view, so `default = []`.
3. **`Cargo.toml`: rlib only, package renamed `gpui-mobile`.** The consumer is the `cdylib`;
   building this crate as `staticlib` + `cdylib` too tripled link time. The `[profile.*]` sections
   were dropped (ignored for a non-root package, and they warned on every build).
4. **`src/android/window.rs`: `gpu_specs`.** The fork's `WgpuRenderer::gpu_specs` returns an
   `Option`; `map` became `and_then`. The only API difference against the fork (rebuilt on upstream
   Zed 933d8d93, which contains gpui-pre 0.3.6's base bcf6582).
5. **`src/android/mod.rs`, `src/android/window.rs`: forced backend.** `force_backend(Some(..))`
   limits the shared GPU context to Vulkan or GL (`create_forced_context`), so the two can be
   compared on a device. Default `None` keeps GPUI's own choice (Vulkan preferred, GL fallback).
6. **`src/android/host.rs`: surface race.** `surface_created` marks the surface as held on the
   caller's (UI) thread. Upstream set that flag only when the render thread processed the command,
   so a `surface_destroyed` arriving first returned without waiting and the framework freed a
   surface the render thread was about to attach.
7. **`src/android/window.rs`: present observer.** `window::set_present_observer(f)` runs
   `f(width, height)` on the render thread after every presented frame. The host uses it to end
   `SurfaceHolder.Callback2.surfaceRedrawNeededAsync` once a frame of the new size is on screen.
8. **`src/android/window.rs`: fling guard switch.** `window::set_fling_guard_enabled(false)` relays
   touches straight to GPUI. The guard (on by default, as upstream) makes the touch that stops a
   fling an ordinary tap, which opened the row under the finger; GPUI's own recognizer already
   stops a fling without a tap.
9. **`src/android/window.rs`: present mode on surface replacement.** `init_window`'s
   `replace_surface` path passed `Some(Mailbox)`, which `replace_surface` applies without checking
   the surface's capabilities: on a Fifo-only surface (GLES) `Surface::configure` failed and the
   next frame panicked the render thread. It now passes `None` (keep the validated mode).
10. **`src/android/platform.rs`: fonts are mapped, not read.** `map_font_file` maps the
    `/system/fonts` files read-only (and skips symlinked duplicates) instead of copying them into
    the heap: native heap after start went from about 78 MB to about 32 MB on API 35, where Noto
    CJK alone is 32 MB. `map_font_file` is public so hosts can register their own font files.
11. **`src/android/mod.rs`, `host.rs`, `window.rs`, `jni.rs`, `fling_guard.rs`, `src/ios/window.rs`:
    touch sample times.** `TouchPoint` carries `timestamp`, `host::motion_event` takes the
    sample's time (the POC host sends each batched historical `ACTION_MOVE` sample with its own
    `getHistoricalEventTime`), and it becomes `gpui::TouchEvent::timestamp`, a field the Zed fork
    added (2026-09-27) so GPUI's recognizer measures fling velocity from when Android sampled the
    touch rather than when a frame got to it. On iOS see patch 18.

12. **`src/android/window.rs`: one catch-up frame after a GL resize.** wgpu's GLES backend keeps
    its EGL window surface across a reconfigure, so on Android the first frame after a resize is
    blitted into a back buffer still at the old size (clipped, the rest black). An idle UI drew
    nothing after it and the wrong frame stayed up; on the GL backend a resize now schedules one
    more forced frame after the next present.

13. **`src/android/host.rs`, `frame_source.rs`: a dying render thread is forgotten.** When
    `gpui-main` ends (a panic unwinding out of GPUI), its `ALooper` is destroyed with it, and the
    vsync thread's next `ALooper_wake` on it aborted the whole process. A guard on the render
    thread now clears both looper pointers as it unwinds, so a GPUI panic freezes the GPUI view
    and leaves the host app running.

14. **`src/android/platform.rs`: clipboard writes reach the host.** The Android clipboard is an
    in-process stub upstream, so a copy never reached the phone's clipboard.
    `platform::set_clipboard_writer(f)` hands every write to the host, which writes Android's
    `ClipboardManager`. Reads (paste) still come from the in-process store.

### iOS (embedded in a React Native view; consumer `packages/gpui-mobile/host/src/ios`)

15. **`Cargo.toml`: one font-kit.** The iOS `font-kit` dependency is the same git revision of
    `zed-industries/font-kit` that the Zed fork's `gpui_wgpu` and `gpui_macos` pin, instead of the
    crates.io `zed-font-kit` upstream took (what `gpui-pre-wgpu` uses), so an iOS build links one
    font-kit.
16. **`src/ios/platform_view.rs`: optional packages.** The `video_player` and `camera` lookups
    are behind their features; with patch 2's `default = []` they did not compile.
17. **`src/ios/window.rs`: `gpu_specs`.** `map` became `and_then`, as in patch 4.
18. **`src/ios/window.rs`, `touch_samples.rs` (new): touch samples.** Every sample carries its
    `UITouch.timestamp` as `TouchEvent::timestamp` (its age on `CACurrentMediaTime`'s clock, taken
    off `Instant::now()`), a move relays each of UIKit's coalesced samples
    (`coalescedTouchesForTouch:`) instead of only the newest, and the newest carries UIKit's
    predicted position (`predictedTouchesForTouch:`). `ios::set_fling_guard_enabled(false)` relays
    touches without `crate::fling_guard`, the iOS twin of patch 8.
19. **`src/ios/platform.rs`: clipboard and URLs.** `ios::set_clipboard_writer(f)` hands every
    clipboard write to the host (the iOS twin of patch 14; the host writes `UIPasteboard` and shows
    "Copied"). `write_to_clipboard` and `open_url` passed a Rust `&str` to
    `stringWithUTF8String:`, which reads to a NUL terminator a `&str` does not have; they build the
    `NSString` from the bytes and length now, and `openURL:` gets the options dictionary it needs.
20. **`src/ios/ffi.rs`: asset source.** `ffi::run_app_with_assets(assets)` is `run_app` with an
    `AssetSource` on the `Application`, the iOS twin of `android::host::start_with_assets`.

21. **`src/ios/panic_guard.rs` (new), `dispatcher.rs`, `ffi.rs`, `window.rs`: a panic stops GPUI,
    not the app.** On iOS every UIKit entry into GPUI is an `extern "C"` function (the display
    link's frame, the touch methods, layout, lifecycle, the GCD trampoline for GPUI's tasks), so a
    panic aborted the whole app. The main-thread entries now run through `panic_guard::contain`:
    the first panic marks GPUI stopped (no entry calls into it again; main-thread tasks are
    forgotten, not dropped, since dropping their futures runs GPUI code), the host's
    `panic_guard::set_stop_observer` hears about it once, and the app keeps running with GPUI's
    last frame. A panic in a background task is caught and logged. The iOS twin of patch 13.
22. **`src/ios/window.rs`: an embedded window is active while the app is.** `is_active` compared
    the view's window with the key window; an embedded view joins the host's window only after
    GPUI opened it, so GPUI read "inactive" at open and capped every animation frame (a fling's
    momentum) at its inactive interval, about 30 fps with 50 ms gaps, until the app next became
    active. Embedded, `is_active` is now `UIApplication.applicationState == active`.

All patches are marked `Ghostex patch` in the source, except the mechanical `timestamp` fields of
patch 11.

## Known upstream limits (not patched)

- The Android library must be linked at API 26 or later: the `ndk` crate's `nativewindow` feature
  links `libnativewindow`, which Android ships from API 26.
- On Android 13+ the system emoji font is COLR v1, which swash cannot draw, and the bundled CBDT
  fallback is only looked up on the `android-activity` path. Hosts register their own CBDT font
  (the POC ships Noto Color Emoji and maps it with `map_font_file`).
- The `android-activity` NativeActivity glue is always linked, so the final library must define
  `android_main` (the POC host defines a stub) or it fails to `dlopen`.
- GLES: the Zed fork's `gpui_wgpu` now asks a native GL adapter for WebGL2 limits and uses the
  WebGL2 instance transport (2026-09-27), so GLES devices are created. The first frame after a
  resize is still drawn at the previous EGL surface size. Vulkan, what phones use, is unaffected.
- Patch 9 is now also fixed at the source: the fork's `replace_surface` keeps a preferred present
  mode only when the new surface supports it (2026-09-27).
- iOS in embedded mode still adds its hidden `UITextView` (the IME scratch view) to GPUI's view, so
  accessibility sees a 1x1 text view inside the GPUI view.
