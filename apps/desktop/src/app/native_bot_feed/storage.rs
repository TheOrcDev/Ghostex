//! The feed's remembered [`FeedPrefs`], one JSON value in client storage (`botFeed` in
//! `packages/client-storage/catalog.ts`), read when the page is made and written on every change
//! through the same door the Docs page uses.

use ghostex_gx_core::bot_feed::FeedPrefs;
use serde_json::json;

use crate::app::gx_store::{read_preference_value, write_client_document_value};
use crate::support_logs::{self, GpuiSupportLog};

const STORED_KEY: &str = "ghostex.botFeed.v1";

/// The stored choices, or the defaults (newest activity, every bot) when none are stored or the
/// value cannot be read.
pub(crate) fn read_feed_prefs() -> FeedPrefs {
    read_preference_value(STORED_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub(crate) fn write_feed_prefs(prefs: &FeedPrefs, cx: &gpui::App) {
    let Ok(raw) = serde_json::to_string(prefs) else {
        return;
    };
    cx.background_executor()
        .spawn(async move {
            // The last saved value stays, so a lost save is written where support looks.
            let details = match write_client_document_value(STORED_KEY, Some(&raw)) {
                Ok(None) => return,
                Ok(Some(bound)) => json!({ "refusedBy": bound }),
                Err(error) => json!({ "error": error }),
            };
            support_logs::append(
                GpuiSupportLog::SidebarRefresh,
                "botFeed.save.failed",
                details,
            );
        })
        .detach();
}
