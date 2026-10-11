use std::{fs, io};

use serde_json::{Value, json};
use tao::{event_loop::EventLoopWindowTarget, window::WindowId};

use crate::*;

impl EditorApp {
    pub(crate) fn handle_daemon_event(
        &mut self,
        event: DaemonEvent,
        target: &EventLoopWindowTarget<DaemonEvent>,
    ) {
        match event {
            DaemonEvent::Request {
                request,
                connection,
            } => self.handle_request(request, connection, target),
            DaemonEvent::WebMessage { window_id, body } => self.handle_web_message(window_id, body),
            DaemonEvent::SaveAllAndExit => self.save_all_sessions_and_exit(),
            DaemonEvent::EnsureWarm => self.ensure_warm_window(target),
        }
    }

    fn handle_request(
        &mut self,
        request: Value,
        connection: ClientConnection,
        target: &EventLoopWindowTarget<DaemonEvent>,
    ) {
        if request.get("v").and_then(Value::as_u64) != Some(PROTOCOL_VERSION) {
            connection.send_error("unsupported protocol version");
            return;
        }
        let Some(request_type) = request.get("type").and_then(Value::as_str) else {
            connection.send_error("missing request type");
            return;
        };
        match request_type {
            "ping" => connection.send(json!({
                "type": "pong",
                "v": PROTOCOL_VERSION,
                "openCount": self.sessions.len(),
                "warm": self.warm_window_is_ready(),
            })),
            "warm" => self.handle_warm(connection, target),
            "open" => self.handle_open(request, connection, target),
            "close" => self.handle_close(request, connection),
            "status" => self.handle_status(connection),
            "front" => self.handle_front(request, connection),
            "retitle" => self.handle_retitle(request),
            "watch" => {
                // Watch subscriptions push openCount changes over a held-open
                // connection so hosts can reflect editor windows the moment
                // they open or close instead of polling with ping.
                connection.send(json!({
                    "type": "watching",
                    "v": PROTOCOL_VERSION,
                    "openCount": self.sessions.len(),
                }));
                self.open_count_watchers.push(connection);
            }
            "shutdown" => self.handle_shutdown(connection),
            _ => connection.send_error("unknown request type"),
        }
    }

    fn handle_warm(
        &mut self,
        connection: ClientConnection,
        target: &EventLoopWindowTarget<DaemonEvent>,
    ) {
        if self.warm_window_is_ready() {
            connection.send(json!({"type": "warmed", "v": PROTOCOL_VERSION}));
            return;
        }
        self.warm_waiters.push(connection);
        self.ensure_warm_window(target);
    }

    fn handle_open(
        &mut self,
        request: Value,
        connection: ClientConnection,
        target: &EventLoopWindowTarget<DaemonEvent>,
    ) {
        let Some(request_id) =
            string_field(&request, "requestId").filter(|value| !value.is_empty())
        else {
            connection.send_error("open request requires requestId");
            return;
        };
        if self.sessions.contains_key(&request_id) {
            connection.send_error("requestId already open");
            return;
        }
        let Some(file_path) = absolute_path_field(&request, "filePath") else {
            connection.send_error("open request requires absolute filePath");
            return;
        };
        let Some(status_file) = absolute_path_field(&request, "statusFile") else {
            connection.send_error("open request requires absolute statusFile");
            return;
        };

        let language = string_field(&request, "language")
            .filter(|value| !value.is_empty())
            .or_else(|| Some(DEFAULT_LANGUAGE.to_string()));
        let originating_session_id = originating_session_id_field(&request);
        let title = string_field(&request, "title")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_TITLE.to_string());
        let initial_text = match fs::read_to_string(&file_path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                connection.send_error(format!("unable to read file: {error}"));
                return;
            }
        };

        let window_id = match self.take_ready_warm_window() {
            Some(window_id) => window_id,
            None => match self.make_editor_window(target) {
                Ok(window_id) => window_id,
                Err(error) => {
                    connection.send_error(format!("unable to open editor window: {error}"));
                    return;
                }
            },
        };

        if let Some(window) = self.windows.get_mut(&window_id) {
            window.window.set_title(&session_window_title(&title));
            window.session_request_id = Some(request_id.clone());
        }

        self.sessions.insert(
            request_id.clone(),
            EditorSession {
                request_id,
                originating_session_id,
                file_path,
                status_file,
                language,
                title,
                initial_text: initial_text.clone(),
                initial_cursor_offset: self.last_cursor_snapshot.as_ref().and_then(|snapshot| {
                    (snapshot.text == initial_text).then_some(snapshot.cursor_offset)
                }),
                latest_draft: initial_text,
                latest_cursor_offset: None,
                opener: connection,
                window_id,
                has_opened: false,
                is_finishing: false,
            },
        );
        self.configure_window_if_ready(window_id);
        // Presentation happens right at open handling, before the configure
        // round-trip through the web layer completes: a warm window already
        // has the composer loaded, so waiting for the "configured" reply only
        // delays window visibility.
        self.present_window(window_id);
        self.notify_open_count_watchers();
        self.ensure_warm_window(target);
    }

    pub(crate) fn notify_open_count_watchers(&mut self) {
        let message = json!({
            "type": "openCountChanged",
            "v": PROTOCOL_VERSION,
            "openCount": self.sessions.len(),
        });
        self.open_count_watchers
            .retain(|watcher| watcher.send_checked(message.clone()));
    }

    fn handle_close(&mut self, request: Value, connection: ClientConnection) {
        let Some(request_id) =
            string_field(&request, "requestId").filter(|value| !value.is_empty())
        else {
            connection.send_error("close request requires requestId");
            return;
        };
        let Some(action) = string_field(&request, "action") else {
            connection.send_error("close request requires action");
            return;
        };
        if !self.sessions.contains_key(&request_id) {
            connection.send_error("unknown requestId");
            return;
        }
        match action.as_str() {
            "save" => {
                connection.send(json!({"type": "ok", "v": PROTOCOL_VERSION}));
                self.request_session_save_and_close(&request_id);
            }
            "cancel" => {
                connection.send(json!({"type": "ok", "v": PROTOCOL_VERSION}));
                self.finish_session(&request_id, CloseAction::Cancel);
            }
            _ => connection.send_error("close action must be save or cancel"),
        }
    }

    fn handle_status(&self, connection: ClientConnection) {
        let sessions: Vec<Value> = self
            .sessions
            .values()
            .map(|session| {
                json!({
                    "requestId": session.request_id,
                    "title": session.title,
                })
            })
            .collect();
        connection.send(json!({
            "type": "status",
            "v": PROTOCOL_VERSION,
            "sessions": sessions,
            "warm": self.warm_window_is_ready(),
        }));
    }

    fn handle_front(&self, request: Value, connection: ClientConnection) {
        let originating_session_id = originating_session_id_field(&request);
        let mut fronted_count = 0usize;
        for session in self.sessions.values() {
            if originating_session_id
                .as_deref()
                .is_some_and(|originating_session_id| {
                    session.originating_session_id.as_deref() != Some(originating_session_id)
                })
            {
                continue;
            }
            let Some(window) = self.windows.get(&session.window_id) else {
                continue;
            };
            window.window.set_visible(true);
            window.window.set_focus();
            fronted_count += 1;
        }
        connection.send(json!({
            "type": "fronted",
            "v": PROTOCOL_VERSION,
            "frontedCount": fronted_count,
            "openCount": self.sessions.len(),
        }));
    }

    fn handle_retitle(&mut self, request: Value) {
        // No-reply notification: the CLI resolves the originating terminal
        // session's display title from gxserver after `open`, so a reply (or
        // an unknown-requestId error for a session that already closed) would
        // only inject noise into the opener's opened/closed message waiters.
        let Some(request_id) =
            string_field(&request, "requestId").filter(|value| !value.is_empty())
        else {
            return;
        };
        let Some(title) = string_field(&request, "title").filter(|value| !value.is_empty()) else {
            return;
        };
        let Some(session) = self.sessions.get_mut(&request_id) else {
            return;
        };
        session.title = title;
        if let Some(window) = self.windows.get(&session.window_id) {
            window
                .window
                .set_title(&session_window_title(&session.title));
        }
    }

    fn handle_shutdown(&mut self, connection: ClientConnection) {
        self.pending_shutdown = true;
        connection.send(json!({"type": "ok", "v": PROTOCOL_VERSION}));
        if self.sessions.is_empty() {
            self.should_exit = true;
        }
    }

    fn handle_web_message(&mut self, window_id: WindowId, body: String) {
        let Ok(message) = serde_json::from_str::<Value>(&body) else {
            return;
        };
        let Some(message_type) = message.get("type").and_then(Value::as_str) else {
            return;
        };
        match message_type {
            "ready" => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.is_ready = true;
                }
                self.configure_window_if_ready(window_id);
                self.notify_warm_waiters_if_ready(window_id);
            }
            "configured" => self.session_configured(window_id),
            "draftUpdate" => {
                if let Some(text) = message.get("text").and_then(Value::as_str) {
                    self.update_session_draft(window_id, text);
                }
                if let Some(cursor_offset) = cursor_offset_field(&message, "cursorOffset") {
                    self.update_session_cursor_offset(window_id, cursor_offset);
                }
            }
            "cursorUpdate" => {
                if let Some(cursor_offset) = cursor_offset_field(&message, "cursorOffset") {
                    self.update_session_cursor_offset(window_id, cursor_offset);
                }
            }
            "saveAndClose" => {
                if let Some(text) = message.get("text").and_then(Value::as_str) {
                    self.update_session_draft(window_id, text);
                }
                if let Some(cursor_offset) = cursor_offset_field(&message, "cursorOffset") {
                    self.update_session_cursor_offset(window_id, cursor_offset);
                }
                if let Some(request_id) = self.request_id_for_window(window_id) {
                    self.finish_session(&request_id, CloseAction::Save);
                }
            }
            "save" => {
                if let Some(text) = message.get("text").and_then(Value::as_str) {
                    self.update_session_draft(window_id, text);
                }
                if let Some(cursor_offset) = cursor_offset_field(&message, "cursorOffset") {
                    self.update_session_cursor_offset(window_id, cursor_offset);
                }
                if let Some(request_id) = self.request_id_for_window(window_id) {
                    self.save_session_draft_without_closing(&request_id);
                }
            }
            "cancel" => {
                if let Some(text) = message.get("text").and_then(Value::as_str) {
                    self.update_session_draft(window_id, text);
                }
                if let Some(cursor_offset) = cursor_offset_field(&message, "cursorOffset") {
                    self.update_session_cursor_offset(window_id, cursor_offset);
                }
                if let Some(request_id) = self.request_id_for_window(window_id) {
                    self.finish_session(&request_id, CloseAction::Cancel);
                }
            }
            "pasteImage" => self.handle_paste_image(window_id, &message),
            "loadImagePreview" => self.handle_load_image_preview(window_id, &message),
            _ => {}
        }
    }

    pub(crate) fn handle_window_close(&mut self, window_id: WindowId) {
        if let Some(request_id) = self.request_id_for_window(window_id) {
            self.request_session_save_and_close(&request_id);
        } else if Some(window_id) == self.warm_window {
            self.warm_window = None;
            self.windows.remove(&window_id);
        }
    }
}
