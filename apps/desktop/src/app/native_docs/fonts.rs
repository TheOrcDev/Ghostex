//! Docs shares the bundled UI faces with the window chrome.

pub(crate) use crate::ui_fonts::{UI_FONT as DOCS_FONT, register};
/// Source mode and code: the app's registered monospace family.
pub(crate) const DOCS_MONO: &str = crate::app::native_chat::fonts::CHAT_MONO;
