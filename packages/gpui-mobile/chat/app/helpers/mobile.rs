//! The phone's answers to helpers whose desktop versions call the operating system, AppKit or the
//! desktop's own gxserver bootstrap files.
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::app::model::GpuiRemoteGxserverRequestTarget;

/// The system's reduce-motion switch; the host reports it at `init` (`ChatInit::reduce_motion`).
pub(crate) fn gpui_macos_reduce_motion_enabled() -> bool {
    crate::mobile::reduce_motion()
}

pub(crate) fn gpui_random_uuid_string() -> Result<String, String> {
    Ok(uuid::Uuid::new_v4().to_string())
}

/// The desktop keeps a few remembered flags under its state directory; on the phone that is the
/// host's data directory.
pub(crate) fn ghostex_state_root() -> PathBuf {
    crate::mobile::data_dir()
}

pub(crate) fn gpui_remote_install_unique_id() -> u128 {
    uuid::Uuid::new_v4().as_u128()
}

/// Writes `item` to the clipboard, and hands the text to the host, whose own clipboard is the one
/// the phone's other apps read (`mobile::set_clipboard_handler`). The desktop also plays its copy
/// sound and shows the copied indicator here.
pub(crate) fn gpui_copy_to_clipboard(item: gpui::ClipboardItem, cx: &mut gpui::App) {
    crate::mobile::copy_to_host_clipboard(&item, cx);
}

/// Window glass blurs the desktop behind a native window; a phone surface has nothing behind it.
pub(crate) fn window_glass_active() -> bool {
    false
}

pub(crate) fn window_glass_active_in(_window: &gpui::Window) -> bool {
    false
}

pub(crate) fn window_glass_active_for(_window: Option<gpui::AnyWindowHandle>) -> bool {
    false
}

#[allow(dead_code)]
pub(crate) const WINDOW_GLASS_MENU_ALPHA: f32 = 0.78;

/// A menu's frosted blur samples what is behind its native window; with glass off it is never asked.
pub(crate) fn apply_frosted_menu_blur(_window: &gpui::Window) {}

/// Never reached with glass off.
pub(crate) fn sidebar_glass_tint() -> gpui::Hsla {
    gpui::transparent_black()
}

pub(crate) fn sync_overlay_window_glass(
    _window: &gpui::Window,
    _main_origin: gpui::Point<gpui::Pixels>,
) {
}

/// No window glass, so a menu keeps its solid fill.
pub(crate) fn frosted_menu_fill(color: gpui::Hsla) -> gpui::Hsla {
    color
}

/// Tooltips draw in a frosted window only under glass; without glass their fill is opaque.
pub(crate) fn frosted_menu_alpha() -> f32 {
    1.0
}

/// The chat view's typed gxserver call (`native_chat/rpc.rs`, the desktop's own file): one
/// `POST /api/<method>` to the gxserver the host named at `init`, answered as `(status, body)`.
pub(crate) fn gxserver_post_typed_operation(
    path: &str,
    params: &Value,
    timeout: Duration,
) -> Result<(u16, String), String> {
    crate::mobile::gxserver_http::post_typed_operation(path, params, timeout)
}

/// A chat on one of the phone's computers: the call goes through the phone's forward to that
/// computer's gxserver (`model::GpuiRemoteGxserverRequestTarget`), as a desktop remote chat's
/// goes through its tunnel.
pub(crate) fn gpui_remote_gxserver_post_typed_operation(
    target: &GpuiRemoteGxserverRequestTarget,
    path: &str,
    params: &Value,
    timeout: Duration,
) -> Result<(u16, String), String> {
    crate::mobile::gxserver_http::post_typed_operation_to(&target.endpoint(), path, params, timeout)
}

/// The same call, answered with the envelope's `result` or its error message.
pub(crate) fn gpui_remote_gxserver_rpc_result(
    target: &GpuiRemoteGxserverRequestTarget,
    path: &str,
    params: &Value,
    timeout: Duration,
) -> Result<Value, String> {
    let (status, body) = gpui_remote_gxserver_post_typed_operation(target, path, params, timeout)?;
    let mut envelope: Value =
        serde_json::from_str(&body).map_err(|_| "gxserver returned invalid JSON.".to_string())?;
    if envelope["ok"] == true && (200..300).contains(&status) {
        return Ok(envelope["result"].take());
    }
    Err(envelope["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("gxserver request failed with HTTP {status}.")))
}
