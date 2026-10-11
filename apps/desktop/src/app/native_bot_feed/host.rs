//! The app side of the feed: whether it may open, the Automations row's toggle, reading the runs
//! afresh when it opens, and drawing the cached page in the view panel.

use super::view::NativeBotFeedView;
use crate::GhostexGpuiApp;
use crate::app::helpers::window_glass_active_in;
use crate::app::model::*;
use gpui::{
    AnyElement, AnyView, AppContext as _, InteractiveElement as _, IntoElement, MouseButton,
    MouseDownEvent, ParentElement as _, StyleRefinement, Styled as _, Window, div,
};

impl GhostexGpuiApp {
    pub(crate) fn bot_feed_showing(&self) -> bool {
        self.active_mode == TitlebarMode::BotFeed
    }

    /// The Automations row: the feed opens as a view tab; a second click closes that tab like its
    /// close button does (the next tab, or the picker when it was the last).
    pub(crate) fn toggle_bot_feed_view(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.bot_feed_showing() {
            self.close_view_tab(TitlebarMode::BotFeed, window, cx);
        } else {
            self.open_view_tab(TitlebarMode::BotFeed, window, cx);
        }
    }

    /// The feed became the view showing (`set_active_mode`, which every way of opening a view goes
    /// through): its runs are read afresh rather than at the page's next 30s tick. The page's
    /// first draw makes and reads it.
    pub(crate) fn native_bot_feed_opened(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(view) = &self.native_bot_feed {
            view.update(cx, |view, cx| view.load(cx));
        }
    }

    /// The view panel's feed page, created (and loaded) on its first draw.
    ///
    /// CDXC:Bots 2026-09-27 WHY:
    /// The page is a cached view like Automate's, so terminal output and other app redraws do not rebuild every run's Markdown; it redraws when it notifies itself or the window glass changes.
    pub(crate) fn render_native_bot_feed_surface(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let view = match self.native_bot_feed.clone() {
            Some(view) => view,
            None => {
                let app = cx.weak_entity();
                let view = cx.new(|cx| {
                    let mut view = NativeBotFeedView::new(app, cx);
                    view.load(cx);
                    view
                });
                self.native_bot_feed = Some(view.clone());
                view
            }
        };
        let glass = window_glass_active_in(window);
        if view.update(cx, |view, _| view.sync(glass)) {
            window.render_view_this_frame(view.entity_id());
        }
        div()
            .id("ghostex-gpui-native-bot-feed-host")
            .size_full()
            .min_w_0()
            .min_h_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.focus_project_editor_surface(TitlebarMode::BotFeed, window, cx);
                }),
            )
            .child(AnyView::from(view).cached(StyleRefinement::default().size_full()))
            .into_any_element()
    }

    /// The settings or theme the feed draws from changed.
    pub(crate) fn native_bot_feed_notify_appearance(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(view) = self.native_bot_feed.as_ref() {
            gpui::App::notify(cx, view.entity_id());
        }
    }
}
