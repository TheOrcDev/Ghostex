//! Only the font registration of the desktop's terminal engine: the chat's monospace text
//! (`native_chat/fonts.rs`, `CHAT_MONO`) draws with the vendored JetBrains Mono Nerd Font faces, and
//! a phone has none of them installed. Same faces, same relative paths as the desktop file.
use gpui::App;

pub(crate) fn register_gpui_terminal_engine_fonts(cx: &App) {
    static REGISTERED: std::sync::Once = std::sync::Once::new();
    REGISTERED.call_once(|| {
        let fonts: Vec<std::borrow::Cow<'static, [u8]>> = vec![
            include_bytes!(
                "../../../.dependencies/ghostty/src/font/res/JetBrainsMonoNerdFont-Regular.ttf"
            )
            .as_slice()
            .into(),
            include_bytes!(
                "../../../.dependencies/ghostty/src/font/res/JetBrainsMonoNerdFont-Bold.ttf"
            )
            .as_slice()
            .into(),
            include_bytes!(
                "../../../.dependencies/ghostty/src/font/res/JetBrainsMonoNerdFont-Italic.ttf"
            )
            .as_slice()
            .into(),
            include_bytes!(
                "../../../.dependencies/ghostty/src/font/res/JetBrainsMonoNerdFont-BoldItalic.ttf"
            )
            .as_slice()
            .into(),
        ];
        if let Err(error) = cx.text_system().add_fonts(fonts) {
            log::error!("chat monospace font registration failed: {error}");
        }
    });
}
