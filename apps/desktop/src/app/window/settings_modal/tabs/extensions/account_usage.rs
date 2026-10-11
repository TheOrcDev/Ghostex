//! `TitlebarAccountUsageSection` (accounts/titlebar-settings-section.tsx (deleted 2026-10-01)): which saved accounts
//! show their usage at the bottom of the sidebar, the same stars as Settings > Accounts.
//!
//! CDXC:Extensions 2026-09-10 DECISION:
//! User: the Extensions page also controls which accounts show usage in the GPUI titlebar, keeping the existing controls on the Accounts page. (The usage meters now sit in the sidebar usage strip; see the Settings 2026-09-20 decision in accounts/mod.rs.)
use super::super::super::super::native_modal_kit::*;
use super::super::super::fields::{
    ListItemStatus, SizedButtonSize, SizedButtonVariant, settings_list_item, settings_section,
    settings_sized_button,
};
use super::super::super::palette::SettingsPalette;
use super::super::accounts::data::{Account, provider_label};
use super::super::accounts::widgets::{account_identity, account_star, account_text};
use super::ExtensionsTab;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    Styled as _, div, px,
};
use gpui_component::{h_flex, v_flex};
use serde_json::json;

impl ExtensionsTab {
    pub(crate) fn render_account_usage_section(
        &mut self,
        p: &SettingsPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hide = self.store.read(cx).bool("hideAccountEmails");
        let (connected, data, error, busy) = {
            let client = self.accounts.read(cx);
            (
                client.connected(cx),
                client.data.clone(),
                client.error.clone(),
                client.busy,
            )
        };
        let mut rows: Vec<AnyElement> = Vec::new();
        if !connected {
            rows.push(settings_list_item(
                p,
                None,
                None,
                "No computer connected",
                Some(
                    div()
                        .child("Connect to a computer to choose its accounts.")
                        .into_any_element(),
                ),
                None,
            ));
        } else {
            if !error.is_empty() {
                let retry = settings_sized_button(
                    p,
                    "account-usage-try-again",
                    "Try again",
                    None,
                    None,
                    SizedButtonVariant::Outline,
                    SizedButtonSize::Sm,
                    busy,
                    None,
                    |page: &mut Self, _window, cx| {
                        page.accounts.update(cx, |client, cx| {
                            client.request(json!({ "operation": "list" }), None, cx)
                        })
                    },
                    cx,
                );
                rows.push(settings_list_item(
                    p,
                    Some(ListItemStatus::Warning),
                    None,
                    "Accounts could not be read",
                    Some(div().child(account_text(&error, hide)).into_any_element()),
                    Some(retry),
                ));
            }
            if data.is_none() && error.is_empty() {
                rows.push(settings_list_item(
                    p,
                    None,
                    None,
                    "Reading saved accounts…",
                    None,
                    None,
                ));
            }
            if let Some(data) = data {
                let mut accounts: Vec<Account> = data
                    .accounts()
                    .into_iter()
                    .filter(Account::registered)
                    .collect();
                accounts.sort_by(|left, right| {
                    super::data::locale_compare(&left.provider(), &right.provider()).then_with(
                        || {
                            left.selector_number()
                                .partial_cmp(&right.selector_number())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        },
                    )
                });
                if accounts.is_empty() {
                    rows.push(settings_list_item(
                        p,
                        None,
                        None,
                        "No saved accounts",
                        Some(
                            div()
                                .child("Add a Claude or Codex account in Settings > Accounts to show its usage here.")
                                .into_any_element(),
                        ),
                        None,
                    ));
                } else {
                    let hairline = hsla(p.hairline);
                    let hover = p.modal.raised;
                    let list = accounts.iter().enumerate().map(|(index, account)| {
                        let star_account = account.clone();
                        let star = account_star(
                            p,
                            account,
                            busy,
                            move |page: &mut Self, _window, cx| {
                                page.toggle_usage_star(&star_account, cx)
                            },
                            cx,
                        );
                        let mut detail = provider_label(&account.provider()).to_string();
                        if !account.email().is_empty() && account.email() != account.name() {
                            detail
                                .push_str(&format!(" · {}", account_text(&account.email(), hide)));
                        }
                        h_flex()
                            .id(SharedString::from(format!(
                                "account-usage-{}",
                                account.id()
                            )))
                            .w_full()
                            .min_h(px(56.0))
                            .px(px(16.0))
                            .py(px(8.0))
                            .gap(px(12.0))
                            .items_center()
                            .when(index > 0, |this| this.border_t_1().border_color(hairline))
                            .hover(move |this| this.bg(hsla(hover)))
                            .child(account_identity(p, account))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .text_size(px(14.0))
                                            .line_height(px(20.0))
                                            .text_color(hsla(p.foreground))
                                            .child(account_text(&account.name(), hide)),
                                    )
                                    .child(
                                        div()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .text_size(px(13.0))
                                            .line_height(px(18.57))
                                            .text_color(hsla(p.muted))
                                            .child(detail),
                                    ),
                            )
                            .child(star)
                            .into_any_element()
                    });
                    rows.push(v_flex().w_full().children(list).into_any_element());
                }
            }
        }
        settings_section(
            p,
            "Account usage in the sidebar",
            Some("Star the accounts whose usage you want to see at the bottom of the desktop sidebar. These are the same stars as in Settings > Accounts.".into()),
            None,
            rows,
        )
        .map(IntoElement::into_any_element)
        .unwrap_or_else(|| div().into_any_element())
    }

    /// `AccountTitlebarStar` on this section.
    fn toggle_usage_star(&mut self, account: &Account, cx: &mut Context<Self>) {
        let shown = !account.show_in_titlebar();
        let mut changed = account.raw.clone();
        changed["showInTitlebar"] = json!(shown);
        let store = self.store.clone();
        self.accounts.update(cx, |client, cx| {
            client.request(
                json!({ "operation": "setTitlebar", "id": account.id(), "shown": shown }),
                Some(Box::new(move |ok, cx| {
                    if ok {
                        super::super::super::store::store_host_message(
                            &store,
                            json!({ "type": "accountTitlebarChanged", "machineId": "local", "account": changed }),
                            cx,
                        );
                    }
                })),
                cx,
            )
        });
    }
}
