use std::path::Path;

/// Media files clicked in a terminal open in the system app when their "Images / Videos / Audio
/// open in" setting says so or the Files view cannot show them; the rest go through the chat
/// file routing, which opens them in Files. Extension-only so the decision is cross-platform; the
/// caller separately verifies that the resolved path is a real local file before launching it.
pub(crate) fn gpui_terminal_file_opens_with_os_default(path: &Path) -> bool {
    crate::app::native_docs::entry::media_file_opens_in_files(path) == Some(false)
}
