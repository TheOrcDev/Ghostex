//! CDXC:Theming 2026-09-14 DECISION:
//! User: the toolbar below agent terminals must look correct in light mode.
//! Resolve its pill, controls and menus from the app appearance, independently of the terminal content theme.

use gpui::{Hsla, Rgba};

use crate::app::helpers::{chrome_color, window_glass_active};
use crate::app::native_chat::appearance::ChatAppearance;

/// CDXC:Theming 2026-09-23 DECISION:
/// User: "apply glass effect exactly like the one we have for the composer in the gpui chat view for the bar in the gpui terminal view". Under window glass the bar's pill and its ghost buttons' hover take their colours from the chat composer's own frosted appearance (`ChatAppearance::on_window_glass`), so the two stay one definition; the prompt-editor button keeps its solid fill like the composer's Send button does.
fn glass_composer_appearance() -> Option<ChatAppearance> {
    window_glass_active()
        .then(|| ChatAppearance::current(&serde_json::Value::Null).on_window_glass(true))
}

pub(super) fn terminal_agent_bar_indicator_background() -> Rgba {
    chrome_color(0xe0e0e0, 0xdbeafe)
}

pub(super) fn terminal_agent_bar_icon_color() -> Rgba {
    chrome_color(0xa6a6a6, 0x626874)
}

pub(super) fn terminal_agent_bar_disabled_icon_color() -> Rgba {
    chrome_color(0x5a5a5a, 0xb0b5bd)
}

pub(super) fn terminal_agent_bar_background() -> Hsla {
    glass_composer_appearance().map_or_else(
        || chrome_color(0x141414, 0xf4f5f7).into(),
        |composer| composer.composer_background,
    )
}

pub(super) fn terminal_agent_bar_border_color() -> Hsla {
    glass_composer_appearance().map_or_else(
        || chrome_color(0x262626, 0xdedfe3).into(),
        |composer| composer.composer_border,
    )
}

/// The hover of the bar's own ghost buttons: the composer toolbar buttons' hover under glass.
/// Rows in the bar's opaque ⋯ menu keep `terminal_agent_bar_hover_background`.
pub(super) fn terminal_agent_bar_button_hover_background() -> Hsla {
    glass_composer_appearance().map_or_else(
        || terminal_agent_bar_hover_background().into(),
        |composer| composer.border,
    )
}

pub(super) fn terminal_agent_bar_hover_background() -> Rgba {
    chrome_color(0x343434, 0xe5e7eb)
}

pub(super) fn terminal_agent_bar_session_id_color() -> Rgba {
    chrome_color(0x6f6f6f, 0x737985)
}

pub(super) fn terminal_agent_bar_session_id_hover_color() -> Rgba {
    chrome_color(0xc4c4c4, 0x303640)
}

pub(super) fn terminal_agent_bar_accent_background() -> Rgba {
    chrome_color(0xe5e5e5, 0x7db8fb)
}

pub(super) fn terminal_agent_bar_accent_hover_background() -> Rgba {
    chrome_color(0xffffff, 0x68aaf5)
}

pub(super) fn terminal_agent_bar_accent_icon_color() -> Rgba {
    chrome_color(0x111111, 0x173e6b)
}

pub(super) fn terminal_agent_bar_menu_background() -> Rgba {
    chrome_color(0x151515, 0xffffff)
}

pub(super) fn terminal_agent_bar_menu_border() -> Rgba {
    chrome_color(0x2a2a2a, 0xdedfe3)
}

pub(super) fn terminal_agent_bar_menu_separator_color() -> Rgba {
    chrome_color(0x222222, 0xe5e7eb)
}

pub(super) fn terminal_agent_bar_menu_text_color() -> Rgba {
    chrome_color(0xd6d6d6, 0x303640)
}
