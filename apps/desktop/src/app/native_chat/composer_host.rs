//! The composer as its own view, so a keystroke redraws the chat box without telling the chat that
//! anything the transcript reads changed.

use gpui::{AnyElement, AppContext as _, Context, IntoElement, Render, WeakEntity, Window, div};

use super::appearance::ChatAppearance;
use super::state::NativeChatView;

/// CDXC:SessionChat 2026-10-01 WHY:
/// The transcript is drawn again whenever the chat itself is notified (transcript_host.rs), and every keystroke notified the chat to repaint the composer, so typing laid out the visible rows on every keystroke (twice, with the unchanged snapshot the chat core also sent then, gx-chat-core core.rs): with an expanded "Worked for" group that was about 20 redraws and 100ms of main-thread time a second in the GPUI web build. The composer is drawn in this view instead, and what only the composer shows (the draft, its pills, the caret's picture, the box's height) notifies this view; the chat still renders around it as its ancestor, which recomputes the composer's inset under the rows, and the transcript is drawn again only if that inset moved. Its state stays on `NativeChatView`, like the transcript's; this view only draws. The maximized composer is drawn by its own window, which follows the chat, so there `notify_composer` notifies the chat.
pub(crate) struct ComposerHost {
    chat: WeakEntity<NativeChatView>,
}

impl Render for ComposerHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.chat.upgrade() {
            Some(chat) => chat.update(cx, |chat, cx| {
                let p = ChatAppearance::current(&chat.snapshot).on_window_glass(
                    crate::app::helpers::window_glass_active_for(chat.main_window),
                );
                chat.render_composer(&p, window, cx)
            }),
            None => div().into_any_element(),
        }
    }
}

impl NativeChatView {
    /// The composer's view, created on the first draw and drawn uncached, so every chat render
    /// draws it too.
    pub(super) fn render_composer_host(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let chat = cx.weak_entity();
        let host = self
            .composer_host
            .get_or_insert_with(|| cx.new(|_| ComposerHost { chat }))
            .clone();
        gpui::AnyView::from(host).into_any_element()
    }

    /// Repaints the composer after a change only the composer shows.
    pub(crate) fn notify_composer(&mut self, cx: &mut Context<Self>) {
        match &self.composer_host {
            Some(host) if self.maximized_window.is_none() => {
                let host = host.entity_id();
                gpui::App::notify(cx, host);
            }
            _ => cx.notify(),
        }
    }
}
