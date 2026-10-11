use crate::*;
use gpui::{AnyWindowHandle, Bounds, EntityId, Pixels};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Where each terminal's model pill was last painted, and in which window, so Option+P opens the
    /// pop-up against it the way a click on the pill does. Keyed by the app as well, because each
    /// workspace window numbers its terminals from 1 (app/workspace_windows/).
    static TERMINAL_MODEL_PILLS: RefCell<HashMap<(EntityId, TerminalSessionId), (Bounds<Pixels>, AnyWindowHandle)>> =
        RefCell::new(HashMap::new());
}

/// Records the terminal model pill's painted frame (content coordinates of `window`).
pub(crate) fn note_terminal_model_pill(
    app: EntityId,
    session_id: TerminalSessionId,
    bounds: Bounds<Pixels>,
    window: AnyWindowHandle,
) {
    TERMINAL_MODEL_PILLS.with_borrow_mut(|pills| {
        pills.insert((app, session_id), (bounds, window));
    });
}

/// The terminal model pill's last painted frame and window.
pub(crate) fn terminal_model_pill(
    app: EntityId,
    session_id: TerminalSessionId,
) -> Option<(Bounds<Pixels>, AnyWindowHandle)> {
    TERMINAL_MODEL_PILLS.with_borrow(|pills| pills.get(&(app, session_id)).copied())
}

impl GhostexGpuiApp {
    /// CDXC:SessionChat 2026-09-24 DECISION:
    /// User: "take away the quick picker ... Instead ... make our pop-up that we have for the models appear when we are in the terminal view ... and also make it controllable with the keyboard from Option P. I don't want to have two interfaces for picking the model." Option+P toggles the composer's model pop-up on a chat session and the same pop-up over a terminal session's model pill; the full-screen picker and its terminal page are gone.
    pub(crate) fn request_focused_session_model_picker(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let Some(session_id) = self.focused_agents_or_companion_shell_session_id() else {
            return false;
        };
        if !self.agents_chat_mode_sessions.contains(&session_id) {
            let Some((bounds, handle)) = terminal_model_pill(cx.entity_id(), session_id) else {
                return false;
            };
            return self.open_terminal_model_menu(session_id, bounds, handle, cx);
        }
        let Some(chat) = self.native_chat_views.get(&session_id).cloned() else {
            return false;
        };
        let handle = chat
            .read(cx)
            .main_window
            .unwrap_or_else(|| gpui::Window::window_handle(window));
        // Deferred so the chat's own window can be updated even when it is the one handling this key.
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                chat.update(cx, |chat, cx| chat.toggle_model_menu(None, window, cx));
            });
        });
        true
    }

    /// Whether a terminal session offers the model pop-up: an agent with a model lineup, and the
    /// Settings switch for the terminal's model picker on.
    ///
    /// CDXC:Hotkeys 2026-09-08 DECISION:
    /// User: offer the quick model and effort picker for Claude and Codex in terminal view, with a setting to turn it off (enabled by default).
    /// This supersedes the chat-only shortcut rule; disabled or unsupported terminals keep their own bindings.
    /// CDXC:Hotkeys 2026-09-09 DECISION: User: extend the quick picker to Cursor, Grok Build and Antigravity.
    /// The 2026-09-24 decision above replaces that picker with the model pop-up; the setting and the agents it covers stay.
    pub(crate) fn terminal_model_menu_available(&self, session_id: TerminalSessionId) -> bool {
        let settings = shared_settings::shared_sidebar_settings_snapshot();
        settings
            .object()
            .get("showQuickModelPickerInTerminal")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
            && !self.agents_chat_mode_sessions.contains(&session_id)
            && matches!(
                self.agents_session_chat_transcript_agent(session_id),
                Some("claude" | "codex" | "cursor" | "grok" | "antigravity")
            )
            && self
                .workspace_terminal_key_for_shell_session(session_id)
                .is_some()
    }

    /// The focused terminal session whose Option+P opens the model pop-up, for the keyboard router.
    pub(crate) fn terminal_model_picker_session(&self) -> Option<TerminalSessionId> {
        let session_id = self.focused_agents_or_companion_shell_session_id()?;
        self.terminal_model_menu_available(session_id)
            .then_some(session_id)
    }

    /// Opens (or closes) the model pop-up for a terminal session, anchored to its model pill. The
    /// pop-up belongs to the session's chat view, which terminal sessions keep warm and hidden; it
    /// is created here when the pool has not made one, and its runtime is woken so the pop-up shows
    /// the session's current model.
    pub(crate) fn open_terminal_model_menu(
        &mut self,
        session_id: TerminalSessionId,
        trigger: Bounds<Pixels>,
        window: AnyWindowHandle,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if !self.terminal_model_menu_available(session_id) {
            return false;
        }
        let Some(chat) = self.ensure_native_chat(session_id, cx) else {
            return false;
        };
        self.resume_native_chat_runtime_for_session(session_id, cx);
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                chat.update(cx, |chat, cx| {
                    // The pop-up hangs off the terminal's model pill, not off this chat's own pane.
                    chat.mark_menu_outside_pane();
                    chat.toggle_model_menu(Some(trigger), window, cx)
                });
            });
        });
        true
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn sync_terminal_model_picker_keyboard_scope(&self) {
        let session = self.terminal_model_picker_session();
        GPUI_KEYBOARD_ROUTER_TARGETS.with(|targets| {
            if let Some(target) = targets
                .borrow_mut()
                .get_mut(&(self.parent_ns_view as usize))
            {
                target.terminal_model_picker_session = session;
            }
        });
    }
}
