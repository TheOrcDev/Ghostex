//! The dialog an app modal that is web content (an extension modal) opens in its place while the
//! optional web runtime is not installed: the Code view prompt's words, Install (or Retry) and Not
//! now, and the install's progress. Once the runtime runs, the waiting modal replaces it.
//! SEE-ALSO: apps/desktop/src/app/web_runtime_prompt_modal_lifecycle.rs (open, refresh, commands),
//! apps/desktop/src/app/helpers/web_runtime.rs (the state and the CDXC:CefRuntime 2026-09-28 decision).
use super::native_modal_kit::*;
use gpui::{
    App, Context, FocusHandle, IntoElement, KeyDownEvent, ParentElement as _, Render, Styled as _,
    Window, div, px,
};
use std::rc::Rc;

pub(crate) const WEB_RUNTIME_PROMPT_MODAL_WIDTH: f32 = 520.0;
/// First-frame height only; the window is resized to the measured layout on the first prepaint.
pub(crate) const WEB_RUNTIME_PROMPT_MODAL_INITIAL_HEIGHT: f32 = 220.0;

/// What the dialog asks its host to do.
pub(crate) enum WebRuntimePromptModalCommand {
    Install,
    Retry,
    /// Not now, Escape or Close: forget the waiting modal.
    Dismiss,
}

pub(crate) type WebRuntimePromptModalHost = Rc<dyn Fn(WebRuntimePromptModalCommand, &mut App)>;

/// What the dialog shows; the host sends a new one whenever the runtime's state changes.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WebRuntimePromptModalContent {
    pub(crate) title: String,
    pub(crate) message: String,
    /// `Some(true)` offers Retry, `Some(false)` Install, `None` only Close (installing).
    pub(crate) retry: Option<bool>,
}

pub(crate) struct GpuiWebRuntimePromptModalWindow {
    host: WebRuntimePromptModalHost,
    palette: ModalPalette,
    content: WebRuntimePromptModalContent,
    fit: ModalFit,
    focus_handle: FocusHandle,
}

impl GpuiWebRuntimePromptModalWindow {
    pub(crate) fn new(
        content: WebRuntimePromptModalContent,
        palette: ModalPalette,
        host: WebRuntimePromptModalHost,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        Self {
            host,
            palette,
            content,
            fit: ModalFit::new(),
            focus_handle,
        }
    }

    pub(crate) fn set_content(
        &mut self,
        content: WebRuntimePromptModalContent,
        cx: &mut Context<Self>,
    ) {
        if self.content != content {
            self.content = content;
            cx.notify();
        }
    }

    fn send(&mut self, command: WebRuntimePromptModalCommand, cx: &mut Context<Self>) {
        (self.host)(command, cx);
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.remove_window();
        self.send(WebRuntimePromptModalCommand::Dismiss, cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.dismiss(window, cx),
            "enter" => match self.content.retry {
                Some(true) => self.send(WebRuntimePromptModalCommand::Retry, cx),
                Some(false) => self.send(WebRuntimePromptModalCommand::Install, cx),
                None => return,
            },
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Render for GpuiWebRuntimePromptModalWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let content = vec![
            modal_header(&p, self.content.title.clone(), None::<String>),
            div()
                .w_full()
                .text_size(px(13.0))
                .line_height(px(20.15))
                .text_color(hsla(p.muted))
                .child(self.content.message.clone())
                .into_any_element(),
        ];
        let close_label = if self.content.retry.is_some() {
            "Not now"
        } else {
            "Close"
        };
        let mut buttons = vec![modal_action_button(
            &p,
            "web-runtime-prompt-dismiss",
            close_label,
            None,
            ModalButtonTone::Neutral,
            false,
            |this: &mut Self, window, cx| this.dismiss(window, cx),
            cx,
        )];
        if let Some(retry) = self.content.retry {
            buttons.push(modal_action_button(
                &p,
                "web-runtime-prompt-install",
                if retry { "Retry" } else { "Install" },
                None,
                ModalButtonTone::Primary,
                false,
                move |this: &mut Self, _window, cx| {
                    this.send(
                        if retry {
                            WebRuntimePromptModalCommand::Retry
                        } else {
                            WebRuntimePromptModalCommand::Install
                        },
                        cx,
                    )
                },
                cx,
            ));
        }
        modal_shell(
            &p,
            "ghostex-gpui-web-runtime-prompt-modal",
            &self.focus_handle,
            &self.fit,
            Self::on_key_down,
            content,
            modal_footer(buttons),
            None,
            cx,
        )
    }
}

impl ModalCornerClose for GpuiWebRuntimePromptModalWindow {
    fn close_from_corner(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }
}
