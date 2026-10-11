//! `libgx_mobile`: the phone's one Rust library.
//!
//! It links two crates and adds nothing of its own:
//!
//! - `ghostex-gx-chat-mobile`: the chat core through UniFFI (`MobileChatCore`), which the phone's
//!   TypeScript chat host drives today. Its bindings are generated from this library, so they load
//!   `gx_mobile`.
//! - `ghostex-gpui-mobile`: the GPUI transcript (the desktop's own `native_chat` and Rust chat
//!   host) embedded in a React Native view: JNI entry points for `dev.ghostex.gpui.GpuiNative` on
//!   Android, the `ghostex_gpui_*` C ABI on iOS.
//!
//! Both crates export their entry points as `#[no_mangle]` C symbols, which a `cdylib` exports and
//! a `staticlib` carries, so re-exporting the crates is all it takes to link them.

pub use ghostex_gpui_mobile as gpui;
pub use gx_chat_mobile as chat_core;
