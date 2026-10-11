//! Preview host for the web runtime prompt dialog an extension modal opens while the optional web
//! runtime is not installed. States: the default install prompt, `installing`, `failed`.
use super::web_runtime_prompt_modal::*;
use gpui::{App, AppContext as _};
use std::rc::Rc;

pub(super) fn open(demo: &super::DemoEnv, cx: &mut App) {
    let content = match demo.state.as_str() {
        "installing" => WebRuntimePromptModalContent {
            title: "Installing web runtime".to_string(),
            message: "Downloading the component…".to_string(),
            retry: None,
        },
        "failed" => WebRuntimePromptModalContent {
            title: "The web runtime needs another try".to_string(),
            message: "Could not download cef-148.0.5+g1a2b3c4+chromium-148.0.7778.97-windows-x64.tar.gz: connection reset.".to_string(),
            retry: Some(true),
        },
        _ => WebRuntimePromptModalContent {
            title: "Web runtime not installed".to_string(),
            message: "Linear Board needs the web runtime, a ~142 MB optional install (one-time).\nWould you like to install it?".to_string(),
            retry: Some(false),
        },
    };
    let host: WebRuntimePromptModalHost = Rc::new(|command, cx: &mut App| match command {
        WebRuntimePromptModalCommand::Install => eprintln!("install"),
        WebRuntimePromptModalCommand::Retry => eprintln!("retry"),
        WebRuntimePromptModalCommand::Dismiss => {
            eprintln!("dismiss");
            cx.quit();
        }
    });
    let palette = demo.palette;
    super::open_modal_window(
        WEB_RUNTIME_PROMPT_MODAL_WIDTH,
        WEB_RUNTIME_PROMPT_MODAL_INITIAL_HEIGHT,
        move |window, cx| {
            cx.new(|cx| GpuiWebRuntimePromptModalWindow::new(content, palette, host, window, cx))
        },
        cx,
    );
}
