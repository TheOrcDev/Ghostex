//! `SoundField` (a sound select with a Play button), `PetPickerField` and `AppIconPickerField`.
use super::super::super::native_modal_kit::*;
use super::super::catalog::{SettingOption, module, settings_catalog};
use super::super::palette::SettingsPalette;
use super::controls::{ButtonVariant, settings_button, settings_icon_button};
use super::row::{
    CONTROL_LANE_WIDTH, PageAction, RowSpec, setting_row, settings_icon, tooltip_text,
};
use super::select::settings_select;
use super::{SettingsPage, icon};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use gpui_component::{h_flex, v_flex};
use serde_json::{Value, json};

/// `COMPLETION_SOUND_OPTIONS`, with `Off` first when the field allows it.
pub(crate) fn sound_options(allow_off: bool) -> Vec<SettingOption> {
    let mut options = Vec::new();
    if allow_off {
        options.push(SettingOption {
            label: "Off".to_string(),
            value: "off".to_string(),
        });
    }
    options
        .extend(settings_catalog().options(module::COMPLETION_SOUND, "COMPLETION_SOUND_OPTIONS"));
    options
}

/// `SoundField`: the select and a Play button (`playCompletionSoundPreview`), which is disabled
/// with a reason while the value is Off.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sound_field<V: SettingsPage>(
    page: &mut V,
    p: &SettingsPalette,
    key: &'static str,
    spec: RowSpec,
    on_reset: Option<PageAction<V>>,
    allow_off: bool,
    value: &str,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let options = sound_options(allow_off);
    let label = spec.label.clone();
    let select = settings_select(
        page,
        p,
        key,
        &options,
        value,
        None,
        false,
        None,
        move |page: &mut V, next, _window, cx| {
            if !allow_off && next == "off" {
                return;
            }
            let store = page.settings_store().clone();
            store.update(cx, |store, cx| store.update_setting(key, json!(next), cx));
        },
        window,
        cx,
    );
    let off = value == "off";
    let sound = value.to_string();
    let play = settings_icon_button(
        p,
        SharedString::from(format!("{key}-play")),
        icon::PLAYER_PLAY,
        16.0,
        32.0,
        ButtonVariant::Outline,
        Some(if off {
            "Choose a sound to preview it.".into()
        } else {
            "Play selected sound".into()
        }),
        off,
        move |page: &mut V, _window, cx| {
            let store = page.settings_store().clone();
            super::super::store::post_store_message(
                &store,
                json!({ "sound": sound, "type": "playCompletionSoundPreview" }),
                cx,
            );
        },
        cx,
    );
    let _ = label;
    let control = h_flex()
        .w(px(CONTROL_LANE_WIDTH))
        .max_w_full()
        .items_center()
        .gap(px(8.0))
        .child(div().flex_1().min_w_0().child(select))
        .child(play)
        .into_any_element();
    setting_row(p, key, spec, on_reset, control, cx)
}

/// `PET_OPTIONS` as `(id, displayName, description)`.
fn pet_options() -> Vec<(String, String, String)> {
    settings_catalog()
        .module_value(module::PETS, "PET_OPTIONS")
        .and_then(Value::as_array)
        .map(|pets| {
            pets.iter()
                .filter_map(|pet| {
                    Some((
                        pet.get("id")?.as_str()?.to_string(),
                        pet.get("displayName")?.as_str()?.to_string(),
                        pet.get("description")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `PetPickerField`: a 64px preview, the pet select and its description.
pub(crate) fn pet_picker_field<V: SettingsPage>(
    page: &mut V,
    p: &SettingsPalette,
    spec: RowSpec,
    on_reset: Option<PageAction<V>>,
    value: &str,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let pets = pet_options();
    let selected = pets
        .iter()
        .find(|(id, _, _)| id == value)
        .or_else(|| pets.first())
        .cloned()
        .unwrap_or_default();
    let options: Vec<SettingOption> = pets
        .iter()
        .map(|(id, name, _)| SettingOption {
            label: name.clone(),
            value: id.clone(),
        })
        .collect();
    let select = settings_select(
        page,
        p,
        "selectedPetId",
        &options,
        &selected.0,
        None,
        false,
        None,
        |page: &mut V, next, _window, cx| {
            let store = page.settings_store().clone();
            store.update(cx, |store, cx| {
                store.update_setting("selectedPetId", json!(next), cx)
            });
        },
        window,
        cx,
    );
    let control = h_flex()
        .w(px(CONTROL_LANE_WIDTH))
        .max_w_full()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .flex_shrink_0()
                .size(px(64.0))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(hsla(p.hairline))
                .bg(hsla(css_fade(p.raised_hover, 0.3)))
                .text_size(px(22.0))
                .text_color(hsla(p.muted))
                .child(
                    selected
                        .1
                        .chars()
                        .next()
                        .map(String::from)
                        .unwrap_or_default(),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(8.0))
                .child(select)
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(13.0))
                        .text_color(hsla(p.muted))
                        .child(selected.2.clone()),
                ),
        )
        .into_any_element();
    setting_row(p, "selectedPetId", spec, on_reset, control, cx)
}

/// `AppIconPickerField` (off while `APP_ICON_CONTROLS_VISIBLE` is false): the selected icon's
/// preview with an X back to the bundled icon, Select Image, and the error box.
#[allow(clippy::too_many_arguments)]
pub(crate) fn app_icon_picker_field<V: SettingsPage>(
    p: &SettingsPalette,
    spec: RowSpec,
    selected_name: Option<String>,
    is_default: bool,
    error: Option<String>,
    on_choose_file: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    on_use_default: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> AnyElement {
    let preview = div()
        .relative()
        .flex_shrink_0()
        .size(px(64.0))
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(hsla(p.hairline))
                .bg(hsla(css_fade(p.raised_hover, 0.3)))
                .child(settings_icon(icon::PHOTO, 28.0, p.muted)),
        )
        .when(!is_default, |this| {
            this.child(
                div()
                    .id("app-icon-use-default")
                    .absolute()
                    .top(px(-8.0))
                    .right(px(-8.0))
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(hsla(p.hairline))
                    .bg(hsla(p.surface))
                    .shadow_sm()
                    .cursor_pointer()
                    .tooltip(tooltip_text("Use default icon"))
                    .on_click(
                        cx.listener(move |page: &mut V, _: &ClickEvent, window, cx| {
                            on_use_default(page, window, cx);
                        }),
                    )
                    .child(settings_icon(icon::X, 14.0, p.muted)),
            )
        });
    let control = v_flex()
        .w_full()
        .gap(px(12.0))
        .child(
            h_flex().items_center().gap(px(12.0)).child(preview).child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(8.0))
                    .child(settings_button(
                        p,
                        "app-icon-select-image",
                        "Select Image",
                        Some(icon::DOWNLOAD),
                        ButtonVariant::Outline,
                        false,
                        None,
                        on_choose_file,
                        cx,
                    ))
                    .child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(13.0))
                            .text_color(hsla(p.muted))
                            .child(if is_default {
                                "Using the bundled Ghostex icon.".to_string()
                            } else {
                                selected_name.unwrap_or_default()
                            }),
                    ),
            ),
        )
        .children(error.map(|error| {
            h_flex()
                .items_start()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .border_1()
                .border_color(hsla(css_fade(p.destructive, 0.4)))
                .bg(hsla(css_fade(p.destructive, 0.1)))
                .text_size(px(13.0))
                .text_color(hsla(p.destructive))
                .child(settings_icon(icon::ALERT_TRIANGLE, 16.0, p.destructive))
                .child(div().min_w_0().child(error))
        }))
        .into_any_element();
    setting_row(p, "appIconSourceId", spec.wide(), None, control, cx)
}
