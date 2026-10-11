//! The UI scale the Linux app runs at. The whole app is an X11 client (see
//! `force_gpui_x11_backend_for_windowed_cef` in `main.rs`), so under a Wayland compositor it
//! runs on XWayland, where nothing in X11 carries the scale the user picked.

use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const GPUI_X11_SCALE_FACTOR: &str = "GPUI_X11_SCALE_FACTOR";

static PINNED_SCALE_FACTOR: OnceLock<f32> = OnceLock::new();

/// CDXC:PlatformSupport 2026-10-01 WHY:
/// Without `Xft.dpi`, GPUI's X11 client derives its scale from the monitor's physical size (RandR millimetres), so a 1920x1080 14-inch panel on Omarchy/Hyprland came out at 1.5x and the chat, sidebar and Settings were far larger than every other app. Under XWayland Ghostex therefore picks the scale itself: `Xft.dpi` when the desktop publishes one (GNOME, KDE; GPUI and Chromium both read it), otherwise Hyprland's focused-monitor scale when `xwayland:force_zero_scaling` hands X clients native pixels (1.0 when Hyprland scales X clients itself), otherwise 1.0, and never the physical-size guess. `GDK_SCALE` is not a source (superseding the 2026-09-30 version): it is GTK's whole-number hint and Omarchy ships it at 2 regardless of the monitor, so a 1.25 monitor drew Ghostex at 2x whenever Hyprland's answer was missing. XWayland counts as detected from the X server's `XWAYLAND` extension, an inherited `WAYLAND_DISPLAY`, or a reachable Hyprland socket, because session launchers (uwsm, systemd units) can drop any one of them. An explicit `GPUI_X11_SCALE_FACTOR` still wins.
/// SEE-ALSO: `append_platform_command_line_switches` in `cef/linux_x11.rs` gives Chromium the same scale.
pub(crate) fn pin_gpui_x11_scale_factor() {
    if env::var(GPUI_X11_SCALE_FACTOR).is_ok_and(|value| !value.trim().is_empty()) {
        return;
    }
    let Some(server) = XServerScaleHints::read() else {
        return;
    };
    let hyprland = HyprlandSocket::find();
    let xwayland =
        server.xwayland || hyprland.is_some() || crate::linux_inherited_wayland_display().is_some();
    if !xwayland || server.has_xft_dpi {
        return;
    }
    let scale = hyprland
        .and_then(|socket| socket.xwayland_scale())
        .unwrap_or(1.0);
    let _ = PINNED_SCALE_FACTOR.set(scale);
    // SAFETY: called from main() before GPUI starts background threads or framework-owned
    // environment readers, so no concurrent environment access is possible.
    unsafe { env::set_var(GPUI_X11_SCALE_FACTOR, scale.to_string()) };
}

/// The scale `pin_gpui_x11_scale_factor` chose, when it chose one.
pub(crate) fn pinned_scale_factor() -> Option<f32> {
    PINNED_SCALE_FACTOR.get().copied()
}

fn parse_scale(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|scale| scale.is_finite() && *scale > 0.0)
}

struct XServerScaleHints {
    xwayland: bool,
    /// Whether the root window's `RESOURCE_MANAGER` string (what `xrdb` loads) sets `Xft.dpi`,
    /// the value GPUI's X11 client reads first.
    has_xft_dpi: bool,
}

impl XServerScaleHints {
    fn read() -> Option<Self> {
        use x11rb::connection::{Connection as _, RequestConnection as _};
        use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

        let (connection, screen_index) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots.get(screen_index)?.root;
        let xwayland = connection
            .extension_information("XWAYLAND")
            .ok()
            .flatten()
            .is_some();
        let has_xft_dpi = connection
            .get_property(
                false,
                root,
                AtomEnum::RESOURCE_MANAGER,
                AtomEnum::STRING,
                0,
                u32::MAX / 4,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some_and(|reply| {
                String::from_utf8_lossy(&reply.value).lines().any(|line| {
                    line.split_once(':').is_some_and(|(key, value)| {
                        key.trim() == "Xft.dpi" && !value.trim().is_empty()
                    })
                })
            });
        Some(Self {
            xwayland,
            has_xft_dpi,
        })
    }
}

/// Hyprland's request socket (`$XDG_RUNTIME_DIR/hypr/<instance>/.socket.sock`, what `hyprctl`
/// talks to).
struct HyprlandSocket(PathBuf);

impl HyprlandSocket {
    /// The instance named by `HYPRLAND_INSTANCE_SIGNATURE`, or, when the launcher dropped that
    /// variable, the newest instance under the runtime directory whose socket answers.
    fn find() -> Option<Self> {
        let hypr_dir = runtime_dir()?.join("hypr");
        if let Some(signature) = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            let socket = Self(hypr_dir.join(signature.trim()).join(".socket.sock"));
            if socket.path().exists() {
                return Some(socket);
            }
        }
        let mut instances = std::fs::read_dir(&hypr_dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join(".socket.sock"))
            .filter(|socket| socket.exists())
            .filter_map(|socket| {
                let modified = std::fs::metadata(&socket).and_then(|m| m.modified()).ok()?;
                Some((modified, socket))
            })
            .collect::<Vec<_>>();
        instances.sort_by(|left, right| right.0.cmp(&left.0));
        instances
            .into_iter()
            .map(|(_, socket)| Self(socket))
            .find(|socket| socket.request("version").is_some())
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// The focused monitor's scale when Hyprland shows X clients at native pixels
    /// (`xwayland:force_zero_scaling`), 1.0 when Hyprland scales X clients itself, and nothing
    /// when Hyprland does not answer. Uses the plain `hyprctl` text format (`bool: true` /
    /// `int: 1`, and `Monitor … scale: 1.25 … focused: yes` blocks).
    fn xwayland_scale(&self) -> Option<f32> {
        let option = self.request("getoption xwayland:force_zero_scaling")?;
        let force_zero_scaling = option.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            match key.trim() {
                "bool" => Some(value.trim() == "true"),
                "int" => value.trim().parse::<i64>().ok().map(|value| value != 0),
                _ => None,
            }
        })?;
        if !force_zero_scaling {
            return Some(1.0);
        }
        let monitors = self.request("monitors")?;
        let mut blocks: Vec<(Option<f32>, bool)> = Vec::new();
        for line in monitors.lines() {
            if line.starts_with("Monitor ") {
                blocks.push((None, false));
                continue;
            }
            let Some(block) = blocks.last_mut() else {
                continue;
            };
            let Some((key, value)) = line.trim().split_once(':') else {
                continue;
            };
            match key.trim() {
                "scale" => block.0 = parse_scale(value),
                "focused" => block.1 = value.trim() == "yes",
                _ => {}
            }
        }
        blocks
            .iter()
            .find(|(_, focused)| *focused)
            .and_then(|(scale, _)| *scale)
            .or_else(|| blocks.iter().find_map(|(scale, _)| *scale))
    }

    fn request(&self, request: &str) -> Option<String> {
        use std::io::{Read as _, Write as _};
        use std::time::Duration;

        let mut stream = std::os::unix::net::UnixStream::connect(self.path()).ok()?;
        let timeout = Some(Duration::from_millis(500));
        stream.set_read_timeout(timeout).ok()?;
        stream.set_write_timeout(timeout).ok()?;
        stream.write_all(request.as_bytes()).ok()?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).ok()?;
        let response = String::from_utf8(response).ok()?;
        (!response.trim().is_empty()).then_some(response)
    }
}

/// `XDG_RUNTIME_DIR`, or `/run/user/<uid>` when the launcher did not pass it on.
fn runtime_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    use std::os::unix::fs::MetadataExt as _;
    let uid = std::fs::metadata("/proc/self").ok()?.uid();
    Some(PathBuf::from(format!("/run/user/{uid}")))
}
