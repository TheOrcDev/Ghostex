//! [`ChatTranscript`]: the root view a phone host shows, one desktop `NativeChatView` in
//! transcript-only mode, full-bleed, for the session it is pointed at.
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement as _, Render,
    Styled as _, Subscription, Window, div,
};
use serde::Serialize;
use serde_json::{Value, json};

use super::summary::ComposerSummary;
use crate::app::model::{GpuiRemoteGxserverRequestTarget, TerminalSessionId};
use crate::app::native_chat::state::{NativeChatConfig, NativeChatEvent, NativeChatView};

/// One session: on the machine the chat was pointed at in [`crate::init`], or on one of the
/// phone's computers ([`SessionRef::on_machine`], reached through
/// [`ChatTranscript::set_machine_endpoint`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRef {
    /// `None` for the `init` machine.
    pub machine_id: Option<String>,
    pub project_id: String,
    pub session_id: String,
}

impl SessionRef {
    pub fn new(project_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            machine_id: None,
            project_id: project_id.into(),
            session_id: session_id.into(),
        }
    }

    pub fn on_machine(
        machine_id: impl Into<String>,
        project_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            machine_id: Some(machine_id.into()),
            project_id: project_id.into(),
            session_id: session_id.into(),
        }
    }
}

/// What the transcript asks its host for. Subscribe with `cx.subscribe(&transcript, ...)`.
#[derive(Clone, Debug)]
pub enum ChatTranscriptEvent {
    /// The composer-side state changed (status, working, send gate, the draft the chat holds).
    Summary(ComposerSummary),
    /// The chat's whole document changed; only sent after
    /// [`ChatTranscript::set_forward_snapshots`]. This is the object the RN card band draws the
    /// working strip, questions, approvals and notices from (the same `snapshot` a
    /// `MobileChatCore` frame carries).
    Snapshot(Arc<Value>),
    /// Something only the app shell can do, exactly as the desktop's chat hands it to its app:
    /// `{"type":"sessionChatHostAction","action":<name>, ...}` (opening a link or file, the stash,
    /// the terminal view, annotate, ...), or an `{"type":"open","modal":...}` request (a Markdown
    /// table, a Mermaid diagram, Settings). `action` is the message's `action`, or its `type` when
    /// it has none.
    HostAction { action: String, message: Value },
    /// A toast to show (a refused send, a failed action, a finished copy).
    Toast { message: String, error: bool },
}

/// What [`ChatTranscript::send_text`] did with the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome", content = "reason")]
pub enum SendOutcome {
    /// Handed to the core's send, which draws the optimistic row and calls gxserver. A delivery
    /// failure comes back as a toast and the text as the summary's `draft`.
    Sent,
    /// The draft was a slash command the core completed in the composer instead of sending (the
    /// summary's `draft` holds the completion).
    CommandCompleted,
    /// No session is open.
    NoSession,
    /// The chat has not read its stored draft yet (`ComposerSummary::composer_ready`).
    NotReady,
    /// Nothing but whitespace.
    Empty,
    /// The previous send is still in flight.
    Pending,
    /// Sending is refused right now; the reason is also toasted.
    Blocked(String),
    /// A queue or compact on a session that cannot queue.
    CannotQueue,
}

/// How [`ChatTranscript::send_text`] submits, the three gestures of the desktop's Send button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SendMode {
    /// Enter / Send.
    #[default]
    Send,
    /// Hold Send: queue behind the working turn.
    Queue,
    /// Option-Send: `/compact`, then queue the text.
    Compact,
}

impl SendMode {
    fn wire(self) -> &'static str {
        match self {
            Self::Send => "send",
            Self::Queue => "queue",
            Self::Compact => "compact",
        }
    }
}

pub struct ChatTranscript {
    chat: Option<Entity<NativeChatView>>,
    session: Option<SessionRef>,
    /// The presentation each chat last published, handed back when its session is opened again so
    /// the transcript draws before its snapshot arrives (the web build's `chat_presentations`).
    presentations: HashMap<SessionRef, Value>,
    summary: Option<ComposerSummary>,
    forward_snapshots: bool,
    last_snapshot: Option<Arc<Value>>,
    chat_subscriptions: Vec<Subscription>,
    next_shell_id: u64,
    /// Each computer's gxserver, as the phone's forward reaches it.
    machines: HashMap<String, GpuiRemoteGxserverRequestTarget>,
    /// The host draws its own composer (the `host_composer` field of `NativeChatView`).
    host_composer: bool,
}

impl EventEmitter<ChatTranscriptEvent> for ChatTranscript {}

/// Creates the transcript view, showing `project_id:session_id` (empty strings open nothing yet).
///
/// Use it as the window's root inside a `gpui_component::Root` (see [`crate::root_view`]), which
/// the chat's tooltips and menus draw into.
pub fn open_transcript(
    _window: &mut Window,
    cx: &mut App,
    project_id: &str,
    session_id: &str,
) -> Entity<ChatTranscript> {
    cx.new(|cx| {
        let mut transcript = ChatTranscript {
            chat: None,
            session: None,
            presentations: HashMap::new(),
            summary: None,
            forward_snapshots: false,
            last_snapshot: None,
            chat_subscriptions: Vec::new(),
            next_shell_id: 1,
            machines: HashMap::new(),
            host_composer: false,
        };
        if !project_id.is_empty() && !session_id.is_empty() {
            transcript.open_session(SessionRef::new(project_id, session_id), cx);
        }
        transcript
    })
}

/// Wraps a view in gpui-component's `Root`, which the chat's tooltips, menus and notifications
/// need as the window's root view.
pub fn root_view(
    view: impl Into<gpui::AnyView>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<gpui_component::Root> {
    let view = view.into();
    cx.new(|cx| gpui_component::Root::new(view, window, cx))
}

impl ChatTranscript {
    /// Shows another session. The previous chat is let go (its conversation stays retained in
    /// the chat host, so switching back draws at once).
    pub fn open_session(&mut self, session: SessionRef, cx: &mut Context<Self>) {
        if self.session.as_ref() == Some(&session) && self.chat.is_some() {
            return;
        }
        self.close_session(cx);
        let shell_session_id = TerminalSessionId(self.next_shell_id);
        self.next_shell_id += 1;
        let remote = session
            .machine_id
            .as_ref()
            .and_then(|machine| self.machines.get(machine).cloned());
        let config = NativeChatConfig {
            machine_id: session
                .machine_id
                .clone()
                .unwrap_or_else(super::init::machine_id),
            project_id: session.project_id.clone(),
            session_id: session.session_id.clone(),
            sidebar_session_id: format!(
                "combined-session:{}:{}",
                session.project_id, session.session_id
            ),
            shell_session_id,
            client_id: format!("{}-{}", super::init::client_name(), uuid::Uuid::new_v4()),
            remote,
            app: None,
            parent_native_view: std::ptr::null_mut(),
            initial_snapshot: None,
            initial_presentation: self.presentations.get(&session).cloned(),
        };
        let host_composer = self.host_composer;
        let chat = cx.new(|cx| {
            let mut view = NativeChatView::new(config, cx);
            view.set_transcript_only(true, cx);
            view.host_composer = host_composer;
            view
        });
        let key = session.clone();
        self.chat_subscriptions.push(
            cx.subscribe(&chat, move |this, _chat, event: &NativeChatEvent, cx| {
                this.chat_event(&key, event, cx)
            }),
        );
        self.chat_subscriptions
            .push(cx.observe(&chat, |this, chat, cx| this.chat_changed(&chat, cx)));
        self.chat = Some(chat);
        self.session = Some(session);
        self.summary = None;
        self.last_snapshot = None;
        cx.notify();
    }

    /// Shows nothing and lets the open chat go.
    pub fn close_session(&mut self, cx: &mut Context<Self>) {
        self.chat_subscriptions.clear();
        if let Some(chat) = self.chat.take() {
            chat.update(cx, |chat, cx| chat.dismiss_windows_for_hidden_pane(cx));
        }
        self.session = None;
        cx.notify();
    }

    /// Where one of the phone's computers' gxserver is: the phone's forward to it on
    /// `127.0.0.1:<local_port>` and its bearer token. Chats opened on that machine afterwards use
    /// it, and an open chat's socket reconnects to it (the desktop's `set_endpoint` for a tunnel).
    pub fn set_machine_endpoint(&mut self, machine_id: &str, local_port: u16, token: &str) {
        let target = GpuiRemoteGxserverRequestTarget {
            local_port,
            token: token.to_string(),
        };
        crate::app::gx_chat::set_endpoint(
            machine_id,
            &format!("http://127.0.0.1:{local_port}"),
            token,
        );
        self.machines.insert(machine_id.to_string(), target);
    }

    /// Whether the host draws the composer (the phone's React Native one), so the core's composer
    /// requests reach it as `composerRequest` host actions. Applies to chats opened afterwards.
    pub fn set_host_composer(&mut self, host_composer: bool) {
        self.host_composer = host_composer;
    }

    /// The host composer's answer to a forwarded `readNativeComposer` read.
    pub fn answer_composer_read(&self, id: Value, text: String, cx: &App) {
        if let Some(chat) = &self.chat {
            chat.read(cx).answer_composer_read(id, text);
        }
    }

    pub fn session(&self) -> Option<&SessionRef> {
        self.session.as_ref()
    }

    /// The composer-side state right now.
    pub fn summary(&self, cx: &App) -> Option<ComposerSummary> {
        self.chat
            .as_ref()
            .map(|chat| ComposerSummary::of(chat.read(cx)))
    }

    /// The chat's whole document right now (see [`ChatTranscriptEvent::Snapshot`]).
    pub fn snapshot(&self, cx: &App) -> Option<Arc<Value>> {
        self.chat
            .as_ref()
            .map(|chat| chat.read(cx).snapshot.clone())
    }

    /// Whether [`ChatTranscriptEvent::Snapshot`] is sent on every document change.
    pub fn set_forward_snapshots(&mut self, forward: bool) {
        self.forward_snapshots = forward;
        self.last_snapshot = None;
    }

    /// The host's composer text changed: the core's draft follows it (durable, synced to the
    /// session's other clients), exactly as the desktop composer's typing does.
    pub fn set_draft(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(chat) = &self.chat else {
            return;
        };
        chat.update(cx, |chat, cx| {
            if chat.draft == text {
                return;
            }
            chat.invoke(json!({"type":"composerExpand","editor":true}), cx);
            chat.draft = text.to_string();
            chat.draft_revision += 1;
            chat.persist_draft(cx);
            cx.notify();
        });
    }

    /// The host's composer lost focus: store the draft now, as the desktop does on blur.
    pub fn save_draft(&mut self, cx: &mut Context<Self>) {
        if let Some(chat) = &self.chat {
            chat.update(cx, |chat, cx| chat.save_draft(cx));
        }
    }

    /// Sends `text` through the core's normal send path, the one the desktop composer's Send runs:
    /// the draft is set to `text` (an `editDraft` at the next revision), then submitted as
    /// `{type: "send", text, draftVersion: {draftId, revision}}`. The core stores the draft as
    /// submitted, pushes it to gxserver, draws the optimistic user row and sends
    /// `sendSessionChatMessage`; its answer clears or restores the draft.
    pub fn send_text(
        &mut self,
        text: &str,
        mode: SendMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SendOutcome {
        let Some(chat) = self.chat.clone() else {
            return SendOutcome::NoSession;
        };
        self.set_draft(text, cx);
        chat.update(cx, |chat, cx| {
            if !chat.composer_ready {
                return SendOutcome::NotReady;
            }
            if chat.draft.trim().is_empty() {
                return SendOutcome::Empty;
            }
            if chat.pending_send {
                return SendOutcome::Pending;
            }
            if mode != SendMode::Send && chat.snapshot["queue"]["capabilities"]["canQueue"] != true
            {
                return SendOutcome::CannotQueue;
            }
            if let Some(reason) = chat.snapshot["sendBlockedReason"].as_str() {
                let reason = reason.to_string();
                // The desktop's own refusal: the core's toast, raised as a host request.
                chat.submit(mode.wire(), window, cx);
                return SendOutcome::Blocked(reason);
            }
            let command = mode == SendMode::Send && chat.snapshot["composerCommand"].is_string();
            chat.submit(mode.wire(), window, cx);
            if command {
                SendOutcome::CommandCompleted
            } else {
                SendOutcome::Sent
            }
        })
    }

    /// Any other core action the host's controls perform (`interrupt`, `composerScroll`,
    /// `composerExpand`, `sendQueue`, `answerQuestion`, ...), as the JSON object the desktop's
    /// controls pass to `invoke`.
    pub fn dispatch_action(&mut self, action: Value, cx: &mut Context<Self>) {
        if let Some(chat) = &self.chat {
            chat.update(cx, |chat, cx| chat.invoke(action, cx));
        }
    }

    /// Scrolls the transcript by `delta` logical pixels (positive shows older rows), for hosts and
    /// tools that scroll without a gesture.
    pub fn scroll_by(&mut self, delta: f32, cx: &mut Context<Self>) {
        if let Some(chat) = &self.chat {
            chat.update(cx, |chat, cx| {
                chat.list.scroll_by(gpui::px(-delta));
                cx.notify();
            });
        }
    }

    /// Jumps to the newest row and follows it again.
    pub fn scroll_to_end(&mut self, cx: &mut Context<Self>) {
        if let Some(chat) = &self.chat {
            chat.update(cx, |chat, cx| {
                chat.list.scroll_to_end();
                chat.list.set_follow_mode(gpui::FollowMode::Tail);
                cx.notify();
            });
        }
    }

    /// How many transcript rows the list holds (for tools and tests).
    pub fn row_count(&self, cx: &App) -> usize {
        self.chat
            .as_ref()
            .map_or(0, |chat| chat.read(cx).list.item_count())
    }

    fn chat_event(
        &mut self,
        session: &SessionRef,
        event: &NativeChatEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            NativeChatEvent::Broker(message) => {
                if message["method"] == "presentation" {
                    self.presentations
                        .insert(session.clone(), message["params"]["state"].clone());
                }
            }
            NativeChatEvent::Host(message) => {
                let kind = message["type"].as_str().unwrap_or_default();
                let action = message["action"].as_str().unwrap_or(kind).to_string();
                if kind == "toast" || action == "toast" {
                    // The view's own toasts carry `message`; the core's app toasts (a refused send)
                    // carry `title` and an optional `description`.
                    let mut text = message["message"]
                        .as_str()
                        .or_else(|| message["title"].as_str())
                        .unwrap_or_default()
                        .to_string();
                    if let Some(description) = message["description"].as_str() {
                        text = format!("{text}\n{description}");
                    }
                    if !text.is_empty() {
                        let error = message["level"] == "error" || message["variant"] == "error";
                        cx.emit(ChatTranscriptEvent::Toast {
                            message: text,
                            error,
                        });
                    }
                    return;
                }
                cx.emit(ChatTranscriptEvent::HostAction {
                    action,
                    message: message.clone(),
                });
            }
            NativeChatEvent::DraftState(_) | NativeChatEvent::ComposerFocused => {}
        }
    }

    /// Every chat notify: forward what changed.
    fn chat_changed(&mut self, chat: &Entity<NativeChatView>, cx: &mut Context<Self>) {
        let view = chat.read(cx);
        let summary = ComposerSummary::of(view);
        let snapshot = view.snapshot.clone();
        if self.summary.as_ref() != Some(&summary) {
            self.summary = Some(summary.clone());
            cx.emit(ChatTranscriptEvent::Summary(summary));
        }
        // The view starts from its own placeholder (a composer placeholder and a few labels) until
        // the core's first document arrives; only a core document, which always carries `view` and
        // `status`, is the object the host's cards and composer read.
        let core_document = snapshot.get("view").is_some() && snapshot.get("status").is_some();
        if self.forward_snapshots
            && core_document
            && !self
                .last_snapshot
                .as_ref()
                .is_some_and(|last| Arc::ptr_eq(last, &snapshot))
        {
            self.last_snapshot = Some(snapshot.clone());
            cx.emit(ChatTranscriptEvent::Snapshot(snapshot));
        }
    }
}

impl Render for ChatTranscript {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .children(self.chat.clone())
    }
}
