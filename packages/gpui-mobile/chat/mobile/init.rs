//! [`init`]: what a host sets up once, before it opens a transcript.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use gpui::App;
use serde_json::{Map, Value};

/// Everything the chat needs from its host, once per process.
#[derive(Clone, Debug)]
pub struct ChatInit {
    /// A private, writable directory. The chat keeps its client storage (drafts, the send outbox,
    /// option pills, the transcript cache a reopened chat draws first) in
    /// `<data_dir>/client-storage.sqlite3`.
    pub data_dir: PathBuf,
    /// The machine the sessions are on, as the desktop names it: `"local"` for the gxserver at
    /// `base_url`. It prefixes the chat's storage keys and keys the chat socket.
    pub machine_id: String,
    /// gxserver's HTTP base, `http://<host>:<port>` (on an emulator or a phone with `adb reverse`,
    /// `http://127.0.0.1:<port>`). The chat socket is `ws://` on the same host and port.
    pub base_url: String,
    /// gxserver's bearer token. Kept in memory only; never logged.
    pub auth_token: String,
    /// The phone's appearance, which the chat's `system` theme follows.
    pub light_appearance: bool,
    /// The system's reduce-motion switch.
    pub reduce_motion: bool,
    /// The desktop's settings object (`settings.json` shape) the chat reads its theme, zoom, width,
    /// verbose and simple modes from; `None` uses every default.
    pub settings: Option<Map<String, Value>>,
    /// Prefix of the chat's client ids, which gxserver shows in its logs next to each send.
    pub client_name: String,
}

impl Default for ChatInit {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::new(),
            machine_id: crate::app::gx_chat::LOCAL_MACHINE_ID.to_string(),
            base_url: String::new(),
            auth_token: String::new(),
            light_appearance: false,
            reduce_motion: false,
            settings: None,
            client_name: "gpui-mobile".to_string(),
        }
    }
}

/// The gxserver the chat's direct calls go to (`native_chat/rpc.rs` through
/// `helpers::gxserver_post_typed_operation`); the chat socket has its own copy in the chat host.
#[derive(Clone)]
pub(crate) struct Endpoint {
    pub(crate) base_url: String,
    pub(crate) auth_token: String,
}

static ENDPOINT: RwLock<Option<Endpoint>> = RwLock::new(None);
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static MACHINE_ID: OnceLock<String> = OnceLock::new();
static CLIENT_NAME: OnceLock<String> = OnceLock::new();
static LIGHT: AtomicBool = AtomicBool::new(false);
static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

/// A host's clipboard, which the chat's copy actions write through as well as through gpui's.
struct ClipboardHandler(std::rc::Rc<dyn Fn(String, &mut App)>);

impl gpui::Global for ClipboardHandler {}

/// Sets up the chat for this process: its fonts, gpui-component, its storage directory, the
/// appearance and settings it draws with, and where gxserver is.
///
/// Call it once, inside `Application::run` (or the platform's embedded run), before
/// [`crate::open_transcript`]. The host passes [`crate::asset_source`] to
/// `Application::with_assets`, because the chat's icons are loaded through the app's asset source.
/// Calling it again updates the endpoint, the appearance and the settings.
pub fn init(cx: &mut App, config: ChatInit) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    let mut first = false;
    ONCE.call_once(|| first = true);
    if first {
        // A host that already set up gpui-component (its own screens) keeps its setup.
        if !cx.has_global::<gpui_component::theme::Theme>() {
            gpui_component::init(cx);
        }
        crate::app::native_chat::fonts::register(cx);
        register_fallback_fonts(cx);
        // A phone's composer is touch: the core words its placeholder and hints for that.
        crate::app::gx_chat::set_touch_composer(true);
        let _ = DATA_DIR.set(config.data_dir.clone());
        let _ = MACHINE_ID.set(config.machine_id.clone());
        let _ = CLIENT_NAME.set(config.client_name.clone());
    }
    set_light_appearance(cx, config.light_appearance);
    REDUCE_MOTION.store(config.reduce_motion, Ordering::Relaxed);
    install_settings(cx, config.settings.unwrap_or_default());
    set_endpoint(&config.base_url, &config.auth_token);
}

/// GPUI's last-resort faces, which a phone does not have.
///
/// CDXC:Mobile 2026-09-27 WHY: the transcript names desktop families (inline code asks for `Menlo`), and when a family is missing GPUI walks its fallback stack (`.ZedMono` = Lilex, `.ZedSans` = IBM Plex Sans, then Helvetica, Noto Sans, Arial, ...). Android has none of them (its Latin face is Roboto), so the first inline code span panicked the GPUI thread ("failed to resolve font 'Menlo' or any of the fallbacks") and the app died. The faces come from the Zed checkout, as the web build's `platform_fonts` does for the same reason.
fn register_fallback_fonts(cx: &mut App) {
    macro_rules! zed_font {
        ($path:literal) => {
            std::borrow::Cow::Borrowed(
                include_bytes!(concat!("../../../../.dependencies/zed/assets/fonts/", $path))
                    .as_slice(),
            )
        };
    }
    let faces = vec![
        zed_font!("ibm-plex-sans/IBMPlexSans-Regular.ttf"),
        zed_font!("ibm-plex-sans/IBMPlexSans-SemiBold.ttf"),
        zed_font!("lilex/Lilex-Regular.ttf"),
        zed_font!("lilex/Lilex-Bold.ttf"),
    ];
    if let Err(error) = cx.text_system().add_fonts(faces) {
        log::error!("could not register GPUI's fallback fonts: {error:#}");
    }
}

/// Points the chat at a gxserver (a new port after a restart, a new token). Open chats reconnect.
pub fn set_endpoint(base_url: &str, auth_token: &str) {
    if let Ok(mut endpoint) = ENDPOINT.write() {
        *endpoint = Some(Endpoint {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token: auth_token.to_string(),
        });
    }
    crate::app::gx_chat::set_endpoint(&machine_id(), base_url, auth_token);
}

/// The phone switched between light and dark; the chat repaints in the new appearance.
pub fn set_light_appearance(cx: &mut App, light: bool) {
    LIGHT.store(light, Ordering::Relaxed);
    // The chat's own colours follow `LIGHT` through `cef::system_page_color_scheme`; tooltips,
    // menus and scrollbars are gpui-component's, themed the way the desktop themes them.
    crate::app::helpers::CHROME_LIGHT_APPEARANCE.store(light, Ordering::Relaxed);
    crate::app::helpers::apply_gpui_component_theme(cx);
    cx.refresh_windows();
}

/// Replaces the settings object the chat draws with (theme, zoom, transcript width, verbose and
/// simple modes). Open chats repaint.
pub fn install_settings(cx: &mut App, settings: Map<String, Value>) {
    crate::shared_settings::install(settings);
    cx.refresh_windows();
}

/// Where the text of every copy action goes besides gpui's own clipboard: the host's clipboard,
/// which is the one the phone's other apps read. Without a handler only gpui's is written.
pub fn set_clipboard_handler(cx: &mut App, handler: impl Fn(String, &mut App) + 'static) {
    cx.set_global(ClipboardHandler(std::rc::Rc::new(handler)));
}

pub(crate) fn copy_to_host_clipboard(item: &gpui::ClipboardItem, cx: &mut App) {
    cx.write_to_clipboard(item.clone());
    let Some(text) = item.text() else {
        return;
    };
    if let Some(handler) = cx
        .try_global::<ClipboardHandler>()
        .map(|handler| handler.0.clone())
    {
        // The handler may touch the app, so it runs outside whatever update is copying.
        cx.defer(move |cx| handler(text, cx));
    }
}

pub(crate) fn endpoint() -> Option<Endpoint> {
    ENDPOINT.read().ok().and_then(|endpoint| endpoint.clone())
}

pub(crate) fn machine_id() -> String {
    MACHINE_ID
        .get()
        .cloned()
        .unwrap_or_else(|| crate::app::gx_chat::LOCAL_MACHINE_ID.to_string())
}

pub(crate) fn client_name() -> String {
    CLIENT_NAME
        .get()
        .cloned()
        .unwrap_or_else(|| "gpui-mobile".to_string())
}

pub(crate) fn data_dir() -> PathBuf {
    DATA_DIR.get().cloned().unwrap_or_default()
}

pub(crate) fn light_appearance() -> bool {
    LIGHT.load(Ordering::Relaxed)
}

pub(crate) fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}
