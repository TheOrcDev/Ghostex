//! The small account pieces the Accounts page and the Extensions page's "Account usage in the
//! sidebar" share: `AccountLogo` (the provider mark in its brand colour), `AccountIdentity` (the
//! logo with the two usage figures), `AccountTitlebarStar`, `CopyCommand`, and Hide emails
//! (`AccountText`).
use super::super::super::super::native_modal_kit::*;
use super::super::super::fields::{SizedButtonVariant, settings_square_button, tooltip_text};
use super::super::super::palette::SettingsPalette;
use super::data::{Account, mask_account_text};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px, svg,
};
use gpui_component::{h_flex, v_flex};

/// Masks or unmasks an input (`type=password`), once per change: `known` remembers what each
/// input was last set to.
pub(crate) fn sync_masked(
    known: &mut std::collections::HashMap<SharedString, bool>,
    id: &SharedString,
    input: &gpui::Entity<gpui_component::input::InputState>,
    masked: bool,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    if known.get(id).copied().unwrap_or(false) == masked {
        return;
    }
    known.insert(id.clone(), masked);
    input.update(cx, |input, cx| input.set_masked(masked, window, cx));
}

/// `AccountText`: the text as shown, masked while Hide emails is on.
pub(crate) fn account_text(text: &str, hide: bool) -> String {
    if hide {
        mask_account_text(text)
    } else {
        text.to_string()
    }
}

/// `AccountLogo`: Claude in its brand orange, Codex in white (dark) or the foreground (light).
///
/// CDXC:AgentProviders 2026-09-08 DECISION:
/// Usage bars remain neutral and selection uses brighter controls. Original provider logos keep their appearance. Account labels sit over larger session icons; sidebar icons have no account indicator.
pub(crate) fn account_logo(p: &SettingsPalette, provider: &str, size: f32) -> AnyElement {
    let (path, color) = if provider == "codex" {
        (
            "agent-icons/codex.svg",
            if p.light {
                p.foreground
            } else {
                gpui::rgb(0xffffff)
            },
        )
    } else {
        ("agent-icons/claude.svg", gpui::rgb(0xd97757))
    };
    svg()
        .path(path)
        .flex_shrink_0()
        .size(px(size))
        .text_color(hsla(color))
        .into_any_element()
}

/// `AccountIdentity`: the logo and the two stacked figures (the second at 60%).
pub(crate) fn account_identity(p: &SettingsPalette, account: &Account) -> AnyElement {
    let [first, second] = account.figures();
    let figure = |id: String, label: Option<String>, value: String, faded: bool| {
        div()
            .id(SharedString::from(id))
            .when(faded, |this| this.opacity(0.6))
            .when_some(label, |this, label| this.tooltip(tooltip_text(label)))
            .child(value)
    };
    let id = account.id();
    h_flex()
        .flex_shrink_0()
        .min_w(px(62.0))
        .items_center()
        .gap(px(7.0))
        .child(account_logo(p, &account.provider(), 21.0))
        .child(
            v_flex()
                .font_family(MODAL_MONO_FONT)
                .text_size(px(11.0))
                .line_height(px(14.3))
                .text_color(hsla(p.foreground))
                .child(figure(
                    format!("account-{id}-figure-1"),
                    first.0,
                    first.1,
                    false,
                ))
                .child(figure(
                    format!("account-{id}-figure-2"),
                    second.0,
                    second.1,
                    true,
                )),
        )
        .into_any_element()
}

/// `AccountTitlebarStar`: shows or hides the account's usage in the sidebar.
///
/// CDXC:AgentProviders 2026-09-08 DECISION:
/// User: a star with the tooltip "Show this account's stats in the titlebar" pins each account independently.
/// The tooltip says "in the sidebar" (as the React star already did when it was deleted) because the usage meters moved from the titlebar to the sidebar usage strip; see the Settings 2026-09-20 decision in mod.rs.
pub(crate) fn account_star<V: 'static>(
    p: &SettingsPalette,
    account: &Account,
    busy: bool,
    on_toggle: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> AnyElement {
    let shown = account.show_in_titlebar();
    let label = if shown {
        "Hide this account's stats from the sidebar"
    } else {
        "Show this account's stats in the sidebar"
    };
    settings_square_button(
        p,
        SharedString::from(format!("account-{}-star", account.id())),
        if shown {
            "modals/settings/star-filled.svg"
        } else {
            "modals/settings/star.svg"
        },
        None,
        SizedButtonVariant::Ghost,
        28.0,
        Some(label.into()),
        busy,
        Some(label.into()),
        on_toggle,
        cx,
    )
}

/// `CopyCommand`: the command in a box with a copy button that shows a check for 1.8s.
pub(crate) fn copy_command<V: 'static>(
    p: &SettingsPalette,
    id: impl Into<SharedString>,
    command: &str,
    copied: bool,
    on_copy: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> AnyElement {
    let id: SharedString = id.into();
    let tooltip: SharedString = if copied {
        "Copied".into()
    } else {
        "Copy command".into()
    };
    h_flex()
        .w_full()
        .items_center()
        .gap(px(12.0))
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(7.0))
        .border_1()
        .border_color(hsla(p.hairline))
        .bg(hsla(if p.glass {
            p.modal.solid_surface
        } else {
            p.surface
        }))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .font_family(MODAL_MONO_FONT)
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(hsla(p.foreground))
                .child(command.to_string()),
        )
        .child(settings_square_button(
            p,
            SharedString::from(format!("{id}-copy")),
            if copied {
                super::super::super::fields::CHECK_ICON
            } else {
                "modals/settings/copy.svg"
            },
            None,
            SizedButtonVariant::Ghost,
            28.0,
            Some(tooltip),
            false,
            None,
            on_copy,
            cx,
        ))
        .into_any_element()
}
