//! `libghostex_gpui_mobile.so`: a GPUI surface embedded in a React Native view.
//!
//! The React Native side (`apps/gpui-mobile-poc/app/modules/gpui-view`) owns a `SurfaceView` or
//! `TextureView` inside an Expo view and forwards its `Surface`, its touches and the Activity's
//! pause/resume here. GPUI runs on gpui-mobile's process-lived `gpui-main` render thread
//! (`.dependencies/gpui-mobile`, host-driven mode) and draws only inside that surface.
//!
//! Pieces:
//!
//! - [`runtime`]: starts GPUI once per process, opens the one window and pumps commands into it.
//! - [`root`]: **the swap point**. [`root::build_root`] returns the view the window shows and the
//!   handler for the commands addressed to it: the chat transcript (`chat_root`, the desktop's
//!   own renderer from `ghostex-gpui-mobile-chat`), or the synthetic `demo` list.
//! - [`events`]: Rust to host messages (JSON), delivered as `onGpuiEvent` on the React Native view.
//! - [`host_view`]: the window's root view: fills the surface, answers host-level commands
//!   (`ping`), measures frames ([`perf`]) and forwards everything else to the root content.
//! - `android`: the JNI entry points the Kotlin `dev.ghostex.gpui.GpuiNative` object calls.
//! - `ios`: the C ABI (`src/ios/include/ghostex_gpui.h`) the Swift `GpuiView` Expo module calls;
//!   GPUI runs on the main thread there, as a child view controller of the React Native view.
//!
//! The command and event protocol is generic JSON: every message is an object with a `type`.

pub mod config;
pub mod events;
pub mod fonts;
pub mod perf;
pub mod root;
pub mod runtime;

mod chat_root;
mod demo;
mod host_view;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;

pub use config::HostConfig;
pub use events::EventSink;
pub use root::{RootContent, build_root};
