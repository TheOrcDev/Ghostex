//! The desktop's GPUI chat transcript, compiled for phones.
//!
//! This crate is the chat-only, native-target sibling of `apps/gpui-web`: it compiles the desktop
//! app's own chat renderer (`apps/desktop/src/app/native_chat/`) and its Rust chat host
//! (`apps/desktop/src/app/gx_chat/`, which runs `packages/gx-chat-core` and reaches gxserver through
//! `packages/gx-chat-client`) unchanged, through symlinks, and exposes the small API in [`mobile`]
//! that a phone host embeds. On the phone the composer and the cards above it (working strip,
//! questions, approvals, notices) are React Native, so the view draws the transcript region only
//! (`NativeChatView::set_transcript_only`).
//!
//! # Layout
//!
//! CDXC:Mobile 2026-09-27 WHY:
//! The crate root is this folder, not `src/`, because it stands in for `apps/desktop/src/`: the desktop files include shared JSON and fonts through relative paths (`../../../../../packages/...` from `app/native_chat/`), rustc resolves them from the symlink's own folder, and only a tree at the same depth as the desktop's finds them. A `src/` one level deeper breaks a dozen `include_str!`s.
//!
//! - `app/native_chat/`, `app/gx_chat/`: per-file symlinks to the desktop's folders, with twins for
//!   what cannot be shared: `native_chat/binding.rs` (the mobile host binds its view in
//!   `mobile/transcript.rs`), `native_chat/focus.rs` (no Chromium child to hand focus back from),
//!   `native_chat/launch.rs` (only the view's half, lifted), and `gx_chat/storage_backend.rs` (the
//!   phone's own `client-storage.sqlite3`, through `packages/client-storage-native`).
//! - `build.rs` + `extracted-items.txt`: named items lifted byte for byte out of desktop files that
//!   mix portable helpers with native-only code (the web build's mechanism).
//! - `app/helpers/mobile.rs`, `app/mobile_app.rs`, `cef.rs`, `shared_settings.rs`,
//!   `support_logs.rs`, `terminal_gpui_engine.rs`, `assets.rs`: small stand-ins for the desktop
//!   modules the shared files name, each documented with what it answers on a phone.
//! - `mobile/`: the host-facing API.

use gpui::AssetSource as _;

mod app;
// The desktop crate root doubles as a prelude (`use crate::*` in its modules), so the shared files
// expect the same names here.
#[allow(unused_imports)]
mod prelude {
    pub(crate) use anyhow::Result;
    pub(crate) use gpui::prelude::FluentBuilder as _;
    pub(crate) use gpui::{
        Action, AnyElement, App, AppContext as _, Bounds, ClipboardEntry, ClipboardItem,
        ContentMask, DismissEvent, Element, ElementId, Entity, FocusHandle, Focusable as _,
        FontWeight, GlobalElementId, Hitbox, Hsla, Image, InteractiveElement as _, IntoElement,
        KeyBinding, KeyDownEvent, Keystroke, LayoutId, Modifiers, MouseButton, MouseDownEvent,
        MouseUpEvent, ParentElement as _, Pixels, Point, PressureStage, Render, RenderOnce,
        ScrollDelta, ScrollHandle, Size, StatefulInteractiveElement as _, Style, Styled as _,
        Window, WindowBounds, WindowControlArea, WindowOptions, canvas, div, point, px, relative,
        rgb, rgba, size, svg,
    };
    pub(crate) use gpui_component::menu::PopupMenu;
    pub(crate) use gpui_component::scroll::Scrollbar;
    pub(crate) use gpui_component::tooltip::Tooltip;
    pub(crate) use gpui_component::{Root, Selectable, h_flex, v_flex};
    pub(crate) use std::cell::RefCell;
    pub(crate) use std::collections::{HashMap, HashSet};
    pub(crate) use std::ops::Range;
    pub(crate) use std::path::{Path, PathBuf};
    pub(crate) use std::rc::Rc;
    pub(crate) use std::sync::Arc;
    pub(crate) use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    pub(crate) use std::time::Duration;
    pub(crate) use web_time::{Instant, SystemTime, UNIX_EPOCH};
}
#[allow(unused_imports)]
pub(crate) use crate::app::consts::*;
pub(crate) use crate::app::helpers::*;
#[allow(unused_imports)]
pub(crate) use crate::app::hotkeys::*;
pub(crate) use crate::app::mobile_app::GhostexGpuiApp;
pub(crate) use crate::app::model::*;
#[allow(unused_imports)]
pub(crate) use prelude::*;

mod assets;
mod cef;
#[allow(dead_code)]
mod hotkey_label;
pub mod mobile;
mod shared_settings;
mod support_logs;
mod terminal_gpui_engine;

/// The desktop's terminal element owns the chord label the hotkey helpers call; the phone has no
/// terminal, so only the label (the desktop's own `hotkey_label.rs`) is under that name.
mod terminal_element {
    pub(crate) use crate::hotkey_label::terminal_overlay_hotkey_chord_label;
}

/// Linux-only window identity on the desktop, which the chat's child windows pass on. A phone
/// surface has neither.
pub(crate) fn gpui_platform_window_app_id() -> Option<String> {
    None
}

pub(crate) fn gpui_platform_window_icon() -> Option<Arc<image::RgbaImage>> {
    None
}

pub use mobile::*;

/// The asset source the host passes to `Application::with_assets`: the chat's own icons and
/// artwork plus gpui-component's icon set.
pub fn asset_source() -> impl gpui::AssetSource {
    assets::GhostexAssets
}

/// Loads one asset through [`asset_source`], for a host that wraps it in its own source.
pub fn load_asset(path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
    assets::GhostexAssets.load(path)
}
