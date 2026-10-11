//! The terminal bar's model pill: the session's model and reasoning, and the button that opens the
//! same model pop-up the chat composer's pill opens.

use super::palette::*;
use gpui::AnyElement;
use gpui::InteractiveElement as _;
use gpui::IntoElement;
use gpui::MouseButton;
use gpui::MouseDownEvent;
use gpui::ParentElement as _;
use gpui::StatefulInteractiveElement as _;
use gpui::Styled as _;
use gpui::div;
use gpui::prelude::FluentBuilder as _;
use gpui::px;
use gpui::svg;
use gpui_component::h_flex;

use crate::app::session_chat_model_picker::note_terminal_model_pill;
use crate::*;

/// The pill's label before the session's chat view has read the model in use.
const LOADING_LABEL: &str = "Model";

impl GhostexGpuiApp {
    /// CDXC:SessionChat 2026-09-24 DECISION:
    /// User: the model pop-up opens in terminal view too, from "New model button": the terminal bar gains a model pill after the session id showing the model and reasoning in use; a click or Option+P opens the composer's model pop-up above it, and the picks apply to the terminal session exactly as they do in chat.
    pub(super) fn render_terminal_agent_bar_model_pill(
        &self,
        session_id: TerminalSessionId,
        suffix: &str,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        if !self.terminal_model_menu_available(session_id) {
            return None;
        }
        let app_id = cx.entity_id();
        let pill = self
            .native_chat_views
            .get(&session_id)
            .map(|chat| chat.read(cx).snapshot["modelMenu"]["pill"].clone())
            .unwrap_or_default();
        let label = pill["label"]
            .as_str()
            .filter(|label| !label.is_empty())
            .unwrap_or(LOADING_LABEL)
            .to_owned();
        let detail = pill["suffix"]
            .as_str()
            .filter(|detail| !detail.is_empty())
            .map(str::to_owned);
        let tooltip = match crate::app::hotkeys::gpui_configured_hotkey_label("openModelPicker") {
            Some(shortcut) => format!("Model and reasoning ({shortcut})"),
            None => "Model and reasoning".to_owned(),
        };
        let hover = terminal_agent_bar_button_hover_background();
        Some(
            h_flex()
                .id(format!("ghostex-gpui-terminal-agent-bar-model-{suffix}"))
                .role(gpui::Role::Button)
                .aria_label(format!("Model: {label}"))
                .relative()
                .flex_shrink(1.0)
                .min_w_0()
                .max_w(px(220.0))
                .h(px(24.0))
                .px(px(10.0))
                .gap(px(4.0))
                .items_center()
                .rounded_full()
                .text_size(px(12.0))
                .text_color(terminal_agent_bar_menu_text_color())
                .hover(move |style| style.bg(hover))
                .tooltip(move |window, cx| {
                    gpui_component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                        let pill = crate::app::session_chat_model_picker::terminal_model_pill(
                            cx.entity_id(),
                            session_id,
                        );
                        if let Some((bounds, handle)) = pill {
                            this.open_terminal_model_menu(session_id, bounds, handle, cx);
                        }
                    }),
                )
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(label),
                )
                .when_some(detail, |this, detail| {
                    this.child(
                        div()
                            .min_w_0()
                            .flex_shrink(1000.0)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(terminal_agent_bar_session_id_color())
                            .child(detail),
                    )
                })
                .child(
                    svg()
                        .path("titlebar/chevron-down.svg")
                        .flex_shrink_0()
                        .size(px(12.0))
                        .text_color(terminal_agent_bar_icon_color()),
                )
                .child(
                    gpui::canvas(
                        move |bounds, window, _| {
                            note_terminal_model_pill(
                                app_id,
                                session_id,
                                bounds,
                                gpui::Window::window_handle(window),
                            );
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .into_any_element(),
        )
    }
}
