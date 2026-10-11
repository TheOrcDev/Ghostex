//! Fonts the embedded views draw with.
//!
//! Android gives GPUI no system font fallback guarantees (gpui-mobile loads a fixed list of
//! `/system/fonts` files, which differs by vendor and release), so the text families a view names
//! are registered from bytes compiled into the library. DM Sans is the chat's proportional family
//! on the desktop (`apps/desktop/src/app/native_chat/fonts.rs` registers the same static faces).

use gpui::App;

use crate::HostConfig;

/// The proportional family the demo and the chat transcript name.
pub const UI_FAMILY: &str = "DM Sans";

pub fn register(cx: &App, config: &HostConfig) {
    // The chat registers DM Sans (and its mono faces) itself, from the desktop's own font list.
    if config.str("content") == Some("demo") {
        register_ui_family(cx);
    }
    register_emoji(cx, config);
}

/// Colour emoji. Android 13+ ships a COLR v1 system emoji font that swash cannot draw, so the
/// host app passes a CBDT font it bundles (`emojiFontPath`); it is mapped, not read into memory.
/// On iOS CoreText falls back to the system's Apple Color Emoji by itself, so nothing is registered.
fn register_emoji(cx: &App, config: &HostConfig) {
    #[cfg(target_os = "ios")]
    if config.str("emojiFontPath").is_none() {
        return;
    }
    let Some(path) = config.str("emojiFontPath") else {
        log::warn!("no emojiFontPath in the start config; emoji will render as missing glyphs");
        return;
    };
    #[cfg(target_os = "android")]
    match gpui_mobile::android::platform::map_font_file(path) {
        Ok(bytes) => match cx.text_system().add_fonts(vec![bytes]) {
            Ok(()) => log::info!("registered emoji font {path}"),
            Err(error) => log::error!("could not register emoji font {path}: {error:#}"),
        },
        Err(error) => log::error!("could not map emoji font {path}: {error}"),
    }
    #[cfg(not(target_os = "android"))]
    let _ = (cx, path);
}

fn register_ui_family(cx: &App) {
    static REGISTERED: std::sync::Once = std::sync::Once::new();
    REGISTERED.call_once(|| {
        let fonts = vec![
            include_bytes!("../../../../.dependencies/app-fonts/dm-sans/static/400.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../../.dependencies/app-fonts/dm-sans/static/500.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../../.dependencies/app-fonts/dm-sans/static/600.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../../.dependencies/app-fonts/dm-sans/static/700.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../../.dependencies/app-fonts/dm-sans/static/400-italic.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../../.dependencies/app-fonts/dm-sans/static/600-italic.ttf")
                .as_slice()
                .into(),
        ];
        match cx.text_system().add_fonts(fonts) {
            Ok(()) => log::info!("registered {UI_FAMILY}"),
            Err(error) => log::error!("could not register {UI_FAMILY}: {error:#}"),
        }
    });
}
