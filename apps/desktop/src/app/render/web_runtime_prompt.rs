//! What a web view shows in place of its page while the optional web runtime is not installed,
//! being installed or failed: the Code view prompt's card, with Install (or Retry) and the view's
//! Hide tab button (CDXC:CefRuntime 2026-09-28 in app/helpers/web_runtime.rs).

use gpui::AnyElement;
use gpui::FontWeight;
use gpui::InteractiveElement as _;
use gpui::IntoElement;
use gpui::MouseButton;
use gpui::MouseDownEvent;
use gpui::ParentElement as _;
use gpui::Styled as _;
use gpui::div;
use gpui::prelude::FluentBuilder as _;
use gpui::px;
use gpui_component::h_flex;

use crate::app::helpers::web_runtime::*;
use crate::app::helpers::*;
use crate::app::model::*;
use crate::app::render::sleeping_card::{view_card_button, view_card_frame};
use crate::*;

impl GhostexGpuiApp {
    /// The whole-view prompt for the web view `mode`, or `None` while the runtime can show pages.
    pub(crate) fn web_runtime_placeholder_signature(
        &self,
        mode: TitlebarMode,
    ) -> Option<ProjectEditorPlaceholderSignature> {
        let subject = format!("The {} view", mode.tab_label());
        let prompt = web_runtime_install_prompt(&subject)?;
        let mut actions = Vec::new();
        match prompt.action {
            Some(WebRuntimePromptAction::Install) => {
                if titlebar_mode_view_tab_hidden_settings_key(mode).is_some() {
                    actions.push(ProjectEditorPlaceholderAction::HideViewTab);
                }
                actions.push(ProjectEditorPlaceholderAction::InstallWebRuntime);
            }
            Some(WebRuntimePromptAction::Retry) => {
                actions.push(ProjectEditorPlaceholderAction::RetryWebRuntime);
            }
            None => {}
        }
        Some(ProjectEditorPlaceholderSignature {
            mode,
            title: prompt.title,
            message: prompt.message,
            actions,
        })
    }

    /// The same card inside part of a view: an HTML file, drawing or media file in Files (which
    /// adds its Open in system app button), or a titlebar extension popup.
    pub(crate) fn render_web_runtime_prompt_card(
        &self,
        id: &'static str,
        prompt: WebRuntimePrompt,
        open_path: Option<String>,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let ink = chrome_ink();
        let mut buttons = h_flex()
            .mt(px(20.0))
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap(px(8.0));
        if let Some(path) = open_path {
            buttons = buttons.child(
                view_card_button("Open in system app")
                    .id("ghostex-gpui-web-runtime-open-system-app")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            this.native_docs_open_with_system_app(&path, cx);
                        }),
                    ),
            );
        }
        if let Some(action) = prompt.action {
            buttons = buttons.child(
                view_card_button(match action {
                    WebRuntimePromptAction::Install => "Install",
                    WebRuntimePromptAction::Retry => "Retry",
                })
                .id("ghostex-gpui-web-runtime-install")
                .bg(ink.opacity(0.14))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                        match action {
                            WebRuntimePromptAction::Install => this.install_web_runtime(false, cx),
                            WebRuntimePromptAction::Retry => this.retry_web_runtime(cx),
                        }
                    }),
                ),
            );
        }
        let has_title = prompt.title.is_some();
        div()
            .id(id)
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .p(px(16.0))
            .child(
                view_card_frame()
                    .text_center()
                    .child(titlebar_svg_icon(
                        TitlebarMode::Browser.tab_icon(),
                        34.0,
                        ink.opacity(0.8).into(),
                    ))
                    .when_some(prompt.title, |this, title| {
                        this.child(
                            div()
                                .mt(px(8.0))
                                .text_center()
                                .text_size(px(15.0))
                                .line_height(px(21.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(ink.opacity(0.9))
                                .child(title),
                        )
                    })
                    .child(
                        div()
                            .mt(px(if has_title { 6.0 } else { 10.0 }))
                            .max_w_full()
                            .text_center()
                            .text_size(px(12.5))
                            .line_height(px(18.0))
                            .text_color(ink.opacity(0.55))
                            .child(prompt.message),
                    )
                    .child(buttons),
            )
            .into_any_element()
    }
}
