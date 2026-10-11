//! The app's own commands: New Window, Check for Updates, Restart and the two Quits.
//!
//! One list, so the sidebar menu and Quick Access's Commands tab offer the same rows with the same
//! words, and post the same message for the host to perform. Only the desktop app can do any of
//! them; a host that cannot says so (`MenuHost::app_lifecycle`, `QuickAccessData::app_lifecycle`)
//! and the rows are not offered.
//!
//! SEE-ALSO: apps/desktop/src/app/helpers/os_cli/main_menus.rs (the macOS menu bar's Ghostex menu,
//! whose labels these match), apps/desktop/src/app/gx_store/sidebar_more_menu.rs (the host).

/// The message type both surfaces post: `{ type, action }`, `action` being an [`AppLifecycleAction::id`].
pub const APP_LIFECYCLE_MESSAGE_TYPE: &str = "runAppLifecycleAction";

/// One app command as a menu row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppLifecycleAction {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    /// The words Quick Access matches the row by.
    pub search_text: &'static str,
}

/// Menu order, top to bottom.
pub const APP_LIFECYCLE_ACTIONS: [AppLifecycleAction; 5] = [
    AppLifecycleAction {
        id: "newWindow",
        title: "New Window",
        icon: "copy-plus",
        search_text: "New Window open another second window monitor screen",
    },
    AppLifecycleAction {
        id: "checkForUpdates",
        title: "Check for Updates…",
        icon: "download",
        search_text: "Check for Updates update upgrade new version release install",
    },
    AppLifecycleAction {
        id: "restart",
        title: "Restart Ghostex",
        icon: "rotate-clockwise",
        search_text: "Restart Ghostex relaunch reopen app",
    },
    AppLifecycleAction {
        id: "quit",
        title: "Quit Ghostex",
        icon: "x",
        search_text: "Quit Ghostex exit close app",
    },
    AppLifecycleAction {
        id: "quitWithBackgroundServices",
        title: "Quit Ghostex & BG Service",
        icon: "player-stop",
        search_text:
            "Quit Ghostex & BG Service exit stop background services gxserver daemons sessions",
    },
];
