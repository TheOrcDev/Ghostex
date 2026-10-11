/// Shared by every chat dialog child window, so they all get the same native window treatment.
pub(super) mod platform;
mod render;
mod window;
pub(super) use window::SaveMarkdownWindowState;
