//! `useAccounts` (packages/core-ui/accounts/use-accounts.ts (deleted 2026-10-01)) for this computer's gxserver: the last
//! `AgentAccountsState`, the error of the last read, busy and refreshing flags, a read when the page
//! opens (the last reading, then refreshed when `refresh_on_open`), and a quiet re-read every 30
//! seconds while it is open.
//! A newer request supersedes an older one's answer (`generation`).
use super::super::super::store::{SettingsStore, store_gxserver_rpc};
use super::data::AccountsState;
use gpui::{App, Context, Entity, Task};
use serde_json::{Value, json};
use std::time::Duration;

/// `setInterval(refresh, 30000)`.
const POLL_INTERVAL: Duration = Duration::from_secs(30);
/// A refreshed list asks the helpers for live usage, which can take a while.
pub(crate) const ACCOUNTS_TIMEOUT: Duration = Duration::from_secs(120);

/// The owner the modal host's gxserver bootstrap names (`accountSetupOwner()` is its
/// `clientId`).
/// SEE-ALSO: GPUI_SIDEBAR_GXSERVER_CLIENT_ID in apps/desktop/src/app/helpers/os_cli/process_and_constants.rs.
pub(crate) const ACCOUNT_SETUP_OWNER: &str = "ghostex-gpui-sidebar";

pub(crate) struct AccountsClient {
    store: Entity<SettingsStore>,
    pub(crate) data: Option<AccountsState>,
    pub(crate) error: String,
    pub(crate) busy: bool,
    pub(crate) refreshing: bool,
    generation: u64,
    pending: bool,
    active: bool,
    refresh_on_open: bool,
    poll: Option<Task<()>>,
}

impl AccountsClient {
    pub(crate) fn new(
        store: Entity<SettingsStore>,
        refresh_on_open: bool,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            store,
            data: None,
            error: String::new(),
            busy: false,
            refreshing: false,
            generation: 0,
            pending: false,
            active: false,
            refresh_on_open,
            poll: None,
        }
    }

    /// Whether this host reaches gxserver (the React `getAccountsConnections()` had one).
    pub(crate) fn connected(&self, cx: &App) -> bool {
        self.store.read(cx).request().gxserver_rpc_available
    }

    /// The page opened or closed.
    pub(crate) fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active == active {
            return;
        }
        self.active = active;
        if !active || !self.connected(cx) {
            self.poll = None;
            self.generation += 1;
            self.busy = false;
            self.refreshing = false;
            self.pending = false;
            cx.notify();
            return;
        }
        let refresh = self.refresh_on_open;
        // The last reading first, so the page shows its accounts at once, then the read the page
        // opens with (see `cached_list` in server/src/accounts/endpoint.rs).
        let this = cx.weak_entity();
        self.request(
            json!({ "operation": "list", "cachedOnly": true }),
            Some(Box::new(move |_, cx| {
                let _ = this.update(cx, |client, cx| {
                    if client.active {
                        client.request(
                            json!({ "operation": "list", "refresh": refresh }),
                            None,
                            cx,
                        );
                    }
                });
            })),
            cx,
        );
        self.poll = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL_INTERVAL).await;
                let Ok(()) = this.update(cx, |client, cx| {
                    if !client.pending {
                        client.request(json!({ "operation": "list" }), None, cx);
                    }
                }) else {
                    break;
                };
            }
        }));
    }

    /// `request(params)`: runs an operation whose answer is the new state; `done` gets whether it
    /// succeeded.
    /// `window.addEventListener('focus', refresh)`: a quiet re-read when the window comes back,
    /// unless one is already running.
    pub(crate) fn refresh_on_focus(&mut self, cx: &mut Context<Self>) {
        if self.active && !self.pending {
            self.request(json!({ "operation": "list" }), None, cx);
        }
    }

    pub(crate) fn request(
        &mut self,
        params: Value,
        done: Option<Box<dyn FnOnce(bool, &mut App)>>,
        cx: &mut Context<Self>,
    ) {
        if !self.connected(cx) {
            if let Some(done) = done {
                done(false, cx);
            }
            return;
        }
        self.generation += 1;
        let generation = self.generation;
        self.pending = true;
        // A re-read of accounts already shown runs in the background and leaves every control
        // usable; only the first read and changes mark the page busy.
        self.busy = !(params["operation"] == "list" && self.data.is_some());
        self.refreshing = params["refresh"].as_bool() == Some(true);
        self.error.clear();
        cx.notify();
        let this = cx.weak_entity();
        store_gxserver_rpc(
            &self.store.clone(),
            "/api/agentAccounts",
            params,
            ACCOUNTS_TIMEOUT,
            move |result, cx| {
                let succeeded = result.is_ok();
                let _ = this.update(cx, |client, cx| {
                    if client.generation != generation {
                        return;
                    }
                    match result {
                        Ok(state) => client.data = Some(AccountsState { raw: state }),
                        Err(error) => {
                            client.error = if error.trim().is_empty() {
                                "Account request failed.".to_string()
                            } else {
                                error
                            }
                        }
                    }
                    client.busy = false;
                    client.refreshing = false;
                    client.pending = false;
                    cx.notify();
                });
                if let Some(done) = done {
                    done(succeeded, cx);
                }
            },
            cx,
        );
    }

    /// `connection.request(params)` outside the state (setup status, helper status): the answer
    /// goes to `reply` only.
    pub(crate) fn call(
        this: &Entity<Self>,
        params: Value,
        reply: impl FnOnce(Result<Value, String>, &mut App) + 'static,
        cx: &mut App,
    ) {
        let (store, connected) = {
            let client = this.read(cx);
            (client.store.clone(), client.connected(cx))
        };
        if !connected {
            reply(Err("The computer connection is unavailable.".into()), cx);
            return;
        }
        store_gxserver_rpc(
            &store,
            "/api/agentAccounts",
            params,
            ACCOUNTS_TIMEOUT,
            reply,
            cx,
        );
    }
}
