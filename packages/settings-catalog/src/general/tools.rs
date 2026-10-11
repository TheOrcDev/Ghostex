use crate::data::*;
use crate::json::opt;
use crate::rows::{row, section, Section};

/// CDXC:Extensions 2026-08-30:
/// The built-in view switches are owned by Settings → Extensions and are
/// searchable from that page's own definitions. General no longer claims
/// them, so a query like "kanban" lands on Extensions instead of matching a
/// General page that has no such section to scroll to.
pub(crate) fn file_opening() -> Section {
    section(
        "fileOpening",
        "File opening",
        vec![
            row("markdownFileOpenView", "Markdown files", "Choose whether Markdown links from agent chat open in Files or Code.").options(CHAT_FILE_OPEN_VIEW_OPTIONS),
            row("htmlFileOpenView", "HTML files", "Choose whether HTML links from agent chat open in Files or Code.").options(CHAT_FILE_OPEN_VIEW_OPTIONS),
            row("imageFileOpenTarget", "Images", "Choose whether pictures and SVGs linked in agent chat or the terminal open in Files or the system app.").options(MEDIA_FILE_OPEN_TARGET_OPTIONS),
            row("videoFileOpenTarget", "Videos", "Choose whether videos linked in agent chat or the terminal play in Files or open in the system app. Formats Files cannot play, such as .mp4 and .mov, always open in the system app.").options(MEDIA_FILE_OPEN_TARGET_OPTIONS),
            row("audioFileOpenTarget", "Audio", "Choose whether audio files linked in agent chat or the terminal play in Files or open in the system app. Formats Files cannot play, such as .m4a, always open in the system app.").options(MEDIA_FILE_OPEN_TARGET_OPTIONS),
        ],
    )
}

pub(crate) fn browser() -> Section {
    section(
        "browser",
        "Browser",
        vec![
            row("webLinkOpenTarget", "Open links in", "Open web links from terminal output (Command-click), session chat, and detected dev servers in the project Browser view or the system default browser.").options(WEB_LINK_OPEN_TARGET_OPTIONS),
        ],
    )
}

pub(crate) fn editor() -> Section {
    section(
        "editor",
        "Editor",
        vec![
            row("codeServerLinkVscodeUserConfig", "Use VS Code settings", "Use the VS Code settings from the local VS Code install."),
            row("codeServerUseVscodeInsidersUserConfig", "Use VS Code Insiders settings", "Use the VS Code Insiders user settings directory."),
            row("showUntrackedProjectDiffWhenNoTrackedChanges", "Show untracked lines without tracked changes", "When tracked git diff is +0 -0, show untracked line counts in project headers (Starship-style prompts ignore untracked lines)."),
        ],
    )
}

pub(crate) fn terminal_dev_servers() -> Section {
    section(
        "terminalDevServers",
        "Dev Servers",
        vec![
            row(
                "terminalDevServerDetectionEnabled",
                "Detect running servers in terminals",
                "Detect localhost dev server URLs from terminal output.",
            ),
            row(
                "terminalDevServerIgnoredPortRules",
                "Ignored ports",
                "Hide detected servers on specific ports or inclusive port ranges.",
            )
            .options(&[opt("9229", "9229"), opt("24678-24680", "24678-24680")]),
        ],
    )
}
