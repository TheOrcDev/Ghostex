//! `TerminalDevServerIgnoredPortsField`: the ignored port rules as removable rows, and a box with
//! Add for a port or an inclusive range (packages/shared/ghostex-settings/terminal-dev-servers.ts (deleted 2026-10-01)
//! rules: 1-65535, merged and sorted).
use super::super::super::native_modal_kit::*;
use super::super::palette::SettingsPalette;
use super::controls::{ButtonVariant, settings_button, settings_icon_button};
use super::row::{PageAction, RowSpec, setting_row};
use super::text::settings_text_input;
use super::{FieldStates, SettingsPage, icon};
use gpui::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div, px,
};
use gpui_component::input::{InputEvent, InputState};
use gpui_component::{h_flex, v_flex};
use serde_json::{Value, json};

const KEY: &str = "terminalDevServerIgnoredPortRules";
const INVALID_RULE_MESSAGE: &str = "Enter a port (e.g. 9229) or a range (e.g. 24678-24680).";

pub(crate) struct IgnoredPortsState {
    input: Entity<InputState>,
    error: Option<String>,
}

/// `parseTerminalDevServerPortRule`: `(lower, upper)`.
fn parse_rule(value: &str) -> Option<(u32, u32)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (lower, upper) = match trimmed.split_once('-') {
        Some((lower, upper)) => (lower.trim(), Some(upper.trim())),
        None => (trimmed, None),
    };
    let digits =
        |text: &str| !text.is_empty() && text.chars().all(|character| character.is_ascii_digit());
    if !digits(lower) || upper.is_some_and(|upper| !digits(upper)) {
        return None;
    }
    let lower: u64 = lower.parse().ok()?;
    let upper: u64 = match upper {
        Some(upper) => upper.parse().ok()?,
        None => lower,
    };
    if lower < 1 || upper > 65535 || lower > upper {
        return None;
    }
    Some((lower as u32, upper as u32))
}

fn canonical((lower, upper): (u32, u32)) -> String {
    if lower == upper {
        lower.to_string()
    } else {
        format!("{lower}-{upper}")
    }
}

/// `normalizeTerminalDevServerIgnoredPortRules`.
pub(crate) fn normalize_ignored_port_rules(rules: &[String]) -> Vec<String> {
    let mut parsed: Vec<(u32, u32)> = rules.iter().filter_map(|rule| parse_rule(rule)).collect();
    parsed.sort();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (lower, upper) in parsed {
        match merged.last_mut() {
            Some(previous) if lower <= previous.1 + 1 => previous.1 = previous.1.max(upper),
            _ => merged.push((lower, upper)),
        }
    }
    merged.into_iter().map(canonical).collect()
}

fn read_rules(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|rules| {
            rules
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn save_rules<V: SettingsPage>(page: &mut V, rules: Vec<String>, cx: &mut Context<V>) {
    let store = page.settings_store().clone();
    store.update(cx, |store, cx| store.update_setting(KEY, json!(rules), cx));
}

fn add_rule<V: SettingsPage>(page: &mut V, window: &mut Window, cx: &mut Context<V>) {
    let Some(state) = page.field_states().ignored_ports.as_mut() else {
        return;
    };
    let text = state.input.read(cx).value().to_string();
    let Some(rule) = parse_rule(&text).map(canonical) else {
        state.error = Some(INVALID_RULE_MESSAGE.to_string());
        cx.notify();
        return;
    };
    state.error = None;
    let input = state.input.clone();
    input.update(cx, |input, cx| input.set_value("", window, cx));
    let store = page.settings_store().clone();
    let mut rules = read_rules(&store.read(cx).value(KEY));
    rules.push(rule);
    save_rules(page, normalize_ignored_port_rules(&rules), cx);
}

impl FieldStates {
    fn ignored_ports_input<V: SettingsPage>(
        page: &mut V,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Entity<InputState> {
        if page.field_states().ignored_ports.is_none() {
            let input =
                cx.new(|cx| InputState::new(window, cx).placeholder("e.g. 9229 or 24678-24680"));
            let subscription = cx.subscribe_in(
                &input,
                window,
                |page: &mut V, _input, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        if let Some(state) = page.field_states().ignored_ports.as_mut()
                            && state.error.take().is_some()
                        {
                            cx.notify();
                        }
                    }
                    InputEvent::PressEnter { .. } => add_rule(page, window, cx),
                    _ => {}
                },
            );
            let states = page.field_states();
            states.subscriptions.push(subscription);
            states.ignored_ports = Some(IgnoredPortsState { input, error: None });
        }
        page.field_states()
            .ignored_ports
            .as_ref()
            .map(|state| state.input.clone())
            .expect("ignored ports state")
    }
}

/// `TerminalDevServerIgnoredPortsField` (a wide row).
pub(crate) fn ignored_ports_field<V: SettingsPage>(
    page: &mut V,
    p: &SettingsPalette,
    spec: RowSpec,
    on_reset: Option<PageAction<V>>,
    value: &Value,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let input = FieldStates::ignored_ports_input(page, window, cx);
    let error = page
        .field_states()
        .ignored_ports
        .as_ref()
        .and_then(|state| state.error.clone());
    let rules = read_rules(value);
    let list: AnyElement = if rules.is_empty() {
        div()
            .text_size(px(14.0))
            .text_color(hsla(p.muted))
            .child("No ignored ports.")
            .into_any_element()
    } else {
        v_flex()
            .gap(px(8.0))
            .children(rules.iter().enumerate().map(|(index, rule)| {
                let remaining: Vec<String> = rules
                    .iter()
                    .filter(|other| *other != rule)
                    .cloned()
                    .collect();
                h_flex()
                    .min_h(px(36.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .rounded(px(MODAL_RADIUS_CONTROL))
                    .border_1()
                    .border_color(hsla(css_fade(p.hairline, 0.7)))
                    .bg(hsla(css_fade(p.raised, 0.4)))
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_family(MODAL_MONO_FONT)
                            .text_size(px(14.0))
                            .child(rule.clone()),
                    )
                    .child(settings_icon_button(
                        p,
                        SharedString::from(format!("ignored-port-remove-{index}")),
                        icon::TRASH,
                        14.0,
                        24.0,
                        ButtonVariant::Ghost,
                        None,
                        false,
                        move |page: &mut V, _window, cx| {
                            save_rules(page, normalize_ignored_port_rules(&remaining), cx);
                        },
                        cx,
                    ))
            }))
            .into_any_element()
    };
    let empty_input = input.read(cx).value().trim().is_empty();
    let control = v_flex()
        .w_full()
        .gap(px(12.0))
        .child(list)
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap(px(8.0))
                .child(settings_text_input(p, &input, None, false, window, cx))
                .child(settings_button(
                    p,
                    "ignored-port-add",
                    "Add",
                    Some(icon::PLUS),
                    ButtonVariant::Outline,
                    empty_input,
                    Some("Enter a port or port range first.".into()),
                    |page: &mut V, window, cx| add_rule(page, window, cx),
                    cx,
                )),
        )
        .children(error.map(|error| {
            div()
                .text_size(px(14.0))
                .text_color(hsla(p.destructive))
                .child(error)
        }))
        .into_any_element();
    setting_row(p, KEY, spec.wide(), on_reset, control, cx)
}
