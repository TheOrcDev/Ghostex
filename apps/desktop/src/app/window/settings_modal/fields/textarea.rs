//! `SettingsTextarea`: the shadcn textarea in the Settings skin (the input tone at 82%, a hairline
//! edge that turns to the focus border, 8px radius, 12px padding, 14/20 text), on GPUI-Kit's
//! `Textarea`. Like `SettingsInput`, it keeps its own buffer while focused so settings echoes never
//! repaint what the user is typing.
use super::super::super::native_modal_kit::*;
use super::super::palette::SettingsPalette;
use super::{FieldStates, SettingsPage};
use gpui::Focusable as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div, px,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};

pub(crate) struct TextareaFieldState {
    pub(crate) input: Entity<TextareaState>,
    /// The value last shown, to refresh the buffer when it changes elsewhere.
    shown: String,
}

impl FieldStates {
    /// The textarea `id`, created on first render with its change handler (`onChange` on every
    /// edit and on blur); the buffer follows `value` whenever the field is not focused.
    /// `rows` is `(min, max)` for the Kit's auto-grow (`field-sizing: content`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn textarea_state<V: SettingsPage>(
        page: &mut V,
        id: &SharedString,
        value: &str,
        placeholder: Option<&str>,
        rows: (usize, usize),
        on_change: impl Fn(&mut V, String, &mut Window, &mut Context<V>) + 'static,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Entity<TextareaState> {
        if !page.field_states().textareas.contains_key(id) {
            let placeholder = placeholder.map(str::to_string);
            let input = cx.new(|cx| {
                let input = TextareaState::new(window, cx)
                    .auto_grow(rows.0.max(1), rows.1.max(rows.0.max(1)))
                    .default_value(value.to_string());
                match placeholder {
                    Some(placeholder) => input.placeholder(placeholder),
                    None => input,
                }
            });
            let subscription = cx.subscribe_in(
                &input,
                window,
                move |page: &mut V, input, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change | InputEvent::Blur) {
                        let text = input.read(cx).value().to_string();
                        on_change(page, text, window, cx);
                    }
                },
            );
            let states = page.field_states();
            states.subscriptions.push(subscription);
            states.textareas.insert(
                id.clone(),
                TextareaFieldState {
                    input,
                    shown: value.to_string(),
                },
            );
        }
        let state = page
            .field_states()
            .textareas
            .get_mut(id)
            .expect("textarea state");
        let input = state.input.clone();
        if state.shown != value {
            state.shown = value.to_string();
            let focused = input.read(cx).focus_handle(cx).is_focused(window);
            if !focused && input.read(cx).value().as_ref() != value {
                let value = value.to_string();
                input.update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
        input
    }
}

/// The Settings textarea skin. `min_height` is the React `min-h` (64px for the stock textarea,
/// taller where a page's class sets one); the field grows with its content.
pub(crate) fn settings_textarea(
    p: &SettingsPalette,
    state: &Entity<TextareaState>,
    min_height: f32,
    monospace: bool,
    disabled: bool,
    window: &Window,
    cx: &gpui::App,
) -> AnyElement {
    let focused = state.read(cx).focus_handle(cx).is_focused(window);
    div()
        .w_full()
        .min_w_0()
        .min_h(px(min_height))
        .px(px(12.0))
        .py(px(12.0))
        .rounded(px(MODAL_RADIUS_CONTROL))
        .border_1()
        .border_color(hsla(if focused { p.focus_border } else { p.hairline }))
        .bg(hsla(p.input_background()))
        .when(disabled, |this| this.opacity(0.5))
        .when(monospace, |this| this.font_family(MODAL_MONO_FONT))
        .child(
            Textarea::new(state)
                .appearance(false)
                .bordered(false)
                .focus_bordered(false)
                .disabled(disabled)
                .placeholder_color(hsla(p.muted))
                .caret_color(hsla(p.foreground))
                .w_full()
                .px(px(0.0))
                .py(px(0.0))
                .text_size(px(14.0))
                .line_height(px(20.0))
                .text_color(hsla(p.foreground)),
        )
        .into_any_element()
}
