use std::{
    collections::HashMap,
    io::Write as _,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use interprocess::local_socket::{Name, Stream};
use serde_json::{Value, json};
use tao::{
    event_loop::EventLoopProxy,
    window::{Window, WindowId},
};
#[cfg(target_os = "windows")]
use wry::WebContext;
use wry::WebView;

pub(crate) const PROTOCOL_VERSION: u64 = 1;
pub(crate) const SOCKET_FILE_NAME: &str = "ghostex-editor.sock";
pub(crate) const DEFAULT_TITLE: &str = "Prompt Editor";
// Linux/Windows have no native window tabs, so the app name and the session
// title (macOS: titlebar + tab title) share the one window title here.
pub(crate) const APP_WINDOW_TITLE: &str = "Ghostex Prompt Editor";
pub(crate) const DEFAULT_LANGUAGE: &str = "markdown";
pub(crate) const CUSTOM_PROTOCOL: &str = "ghostex-editor";

#[derive(Clone)]
pub(crate) struct ClientConnection {
    pub(crate) writer: Arc<Mutex<Stream>>,
}

impl ClientConnection {
    pub(crate) fn send(&self, value: Value) {
        let _ = self.send_checked(value);
    }

    pub(crate) fn send_checked(&self, value: Value) -> bool {
        let Ok(mut line) = serde_json::to_vec(&value) else {
            return false;
        };
        line.push(b'\n');
        let Ok(mut writer) = self.writer.lock() else {
            return false;
        };
        writer.write_all(&line).is_ok() && writer.flush().is_ok()
    }

    pub(crate) fn send_error(&self, message: impl Into<String>) {
        self.send(json!({
            "type": "error",
            "v": PROTOCOL_VERSION,
            "message": message.into(),
        }));
    }
}

pub(crate) enum DaemonEvent {
    Request {
        request: Value,
        connection: ClientConnection,
    },
    WebMessage {
        window_id: WindowId,
        body: String,
    },
    SaveAllAndExit,
    EnsureWarm,
}

pub(crate) struct SocketEndpoint {
    pub(crate) display_path: String,
    pub(crate) name: Name<'static>,
    pub(crate) cleanup_path: Option<PathBuf>,
}

pub(crate) struct EditorApp {
    pub(crate) socket_cleanup_path: Option<PathBuf>,
    pub(crate) web_root: Arc<PathBuf>,
    #[cfg(target_os = "windows")]
    pub(crate) web_context: WebContext,
    pub(crate) proxy: EventLoopProxy<DaemonEvent>,
    pub(crate) windows: HashMap<WindowId, EditorWindow>,
    pub(crate) sessions: HashMap<String, EditorSession>,
    pub(crate) warm_window: Option<WindowId>,
    pub(crate) warm_waiters: Vec<ClientConnection>,
    pub(crate) open_count_watchers: Vec<ClientConnection>,
    pub(crate) pending_shutdown: bool,
    pub(crate) should_exit: bool,
    pub(crate) cascade_offset: i32,
    pub(crate) last_cursor_snapshot: Option<CursorSnapshot>,
}

pub(crate) struct EditorWindow {
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) is_ready: bool,
    pub(crate) session_request_id: Option<String>,
}

pub(crate) struct EditorSession {
    pub(crate) request_id: String,
    pub(crate) originating_session_id: Option<String>,
    pub(crate) file_path: PathBuf,
    pub(crate) status_file: PathBuf,
    pub(crate) language: Option<String>,
    pub(crate) title: String,
    pub(crate) initial_text: String,
    pub(crate) initial_cursor_offset: Option<usize>,
    pub(crate) latest_draft: String,
    pub(crate) latest_cursor_offset: Option<usize>,
    pub(crate) opener: ClientConnection,
    pub(crate) window_id: WindowId,
    pub(crate) has_opened: bool,
    pub(crate) is_finishing: bool,
}

pub(crate) struct CursorSnapshot {
    pub(crate) text: String,
    pub(crate) cursor_offset: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum CloseAction {
    Save,
    Cancel,
}
