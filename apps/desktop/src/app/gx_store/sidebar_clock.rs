//! The sidebar's own once-a-second tick, and the armed Delayed Send / Close After Done labels the
//! chat's working row draws.
//!
//! CDXC:Sidebar 2026-09-21 WHY:
//! The old projection ran a timer once a second for the life of the app and published a clock row
//! per session. Two things rode that tick and nothing else has their
//! cadence: the armed-timer labels every open chat draws, and the re-read of the two client-storage
//! values the menus need, which a titlebar Keep Awake or an agent launched from another surface
//! moves without the store or a publish ever reporting it. The LIST's own labels do not ride it:
//! the view model says exactly when the next one reads differently and the host books one timer for
//! that moment (`gx_store_book_sidebar_deadline`), so an idle sidebar still wakes once an hour
//! rather than three thousand six hundred times.

use std::time::Duration;

use crate::GhostexGpuiApp;

use super::host::now_ms;

/// The projection's own cadence, which the armed countdowns are written against: a label that says
/// `00:31` must read `00:30` a second later.
const TICK: Duration = Duration::from_secs(1);

impl GhostexGpuiApp {
    /// Starts the tick once. Called from the store's bootstrap, whether or not a daemon transport
    /// is configured yet.
    pub(super) fn gx_store_start_sidebar_clock(&mut self, cx: &mut gpui::Context<Self>) {
        if self.gx_store.sidebar_list.clock_started {
            return;
        }
        self.gx_store.sidebar_list.clock_started = true;
        // The wait for the two legs the list needs starts here, with the app
        // (gx_store/sidebar_ready.rs).
        self.gx_store.sidebar_list.ready_recovery_mut().start();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let alive = this
                    .update(cx, |this, cx| {
                        this.gx_store_sidebar_clock_tick(cx);
                    })
                    .is_ok();
                if !alive {
                    return;
                }
            }
        })
        .detach();
    }

    fn gx_store_sidebar_clock_tick(&mut self, cx: &mut gpui::Context<Self>) {
        // A leg the list is still waiting for after a few seconds is declared absent, so the
        // loading skeleton has an end (gx_store/sidebar_ready.rs). First, because everything below
        // asks whether the list is ready.
        self.gx_store_check_sidebar_ready_recovery(cx);
        // The machine tabs: the settings say which machines exist and the connect states say how
        // they are doing, so a tab the sidebar no longer offers falls back to this computer here.
        // It rode the publish until M4d part 2 step 6 and reuses its own answer for a second, so
        // this tick is the same cadence it already had; the connect funnel and the store's
        // bootstrap still force it.
        self.gx_store_sync_remote_clients(false, cx);
        self.gx_store_refresh_armed_actions(cx);
        // The one cadence that notices a Keep Awake armed from the titlebar or an agent launched
        // from another surface (gx_store/sidebar_menus.rs).
        self.gx_store_poll_menu_host(cx);
        // Every periodic counter the store owns. It rode the old projection's comparison until
        // M4d part 2 step 7, which is what a publish drove; this is the cadence that survives the
        // page (gx_store/sidebar_self_check.rs). The records rate-limit themselves to once a
        // minute and write nothing while routine logging is off.
        self.gx_store_sidebar_summaries();
    }

    /// Rebuilds the armed-timer labels of every session and hands the changed ones to the open
    /// chats. Keyed by sidebar session id, in the shape `working_strip.rs` and
    /// `session-chat-working-strip.tsx` draw, so neither renderer changes.
    ///
    /// CDXC:SessionChat 2026-09-21 WHY:
    /// Computed for EVERY session rather than for the drawn rows, which is why it walks the
    /// presentation rather than the sidebar view: a session the machine filter, the Space, a tag
    /// filter or Show Hidden leaves out still has a chat open on it. That is the same reason
    /// the page's own clock walked the zustand store's `sessionsById` instead of the published list.
    pub(super) fn gx_store_refresh_armed_actions(&mut self, cx: &mut gpui::Context<Self>) {
        let now_ms = now_ms();
        let armed = {
            let store = &self.gx_store;
            ghostex_gx_core::armed_actions_by_session(
                &store.core,
                &store.sidebar_list.last_inputs.host,
                now_ms,
            )
        };
        let mut changed = armed.len() != self.native_sidebar.armed_actions.len();
        let mut next = std::collections::HashMap::with_capacity(armed.len());
        for (session_id, actions) in armed {
            let value = serde_json::Value::Array(
                actions
                    .iter()
                    .map(|action| serde_json::json!({ "id": action.id, "label": action.label }))
                    .collect(),
            );
            changed |= self.native_sidebar.armed_actions.get(&session_id) != Some(&value);
            next.insert(session_id, value);
        }
        if !changed {
            return;
        }
        self.native_sidebar.armed_actions = next;
        self.sync_session_chat_armed_actions(cx);
    }
}
