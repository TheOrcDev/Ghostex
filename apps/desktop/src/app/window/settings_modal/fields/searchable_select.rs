//! A `SelectField` with eight or more options: Base UI's select turns into the searchable dropdown
//! there (packages/components/ui/select.tsx (deleted 2026-10-01), `searchable ?? items.length >= 8`), with a chevron on
//! its trigger instead of the up-down selector. The Theme page's terminal palettes use it.
//!
//! The page keeps a [`DropdownState`] per field and passes an accessor for it (the dropdown
//! stores the trigger's bounds and the filter text there).
use super::super::super::native_modal_kit::*;
use super::super::catalog::SettingOption;
use super::super::palette::SettingsPalette;
use super::row::{CONTROL_HEIGHT, PageAction, RowSpec, SELECT_WIDTH, setting_row, settings_icon};
use super::searchable_dropdown::{
    DropdownAlign, DropdownRow, DropdownState, searchable_dropdown, toggle_dropdown,
};
use super::{SettingsPage, icon};
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use gpui_component::h_flex;

/// CDXC:DesignSystem 2026-09-08 DECISION:
/// User: replace dropdowns that usually have lots of items with the wider, rounded Sessions dropdown with search at the top.
/// The trigger and, while open, the searchable dropdown of `options`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn searchable_select<V: SettingsPage>(
    page: &mut V,
    p: &SettingsPalette,
    id: &'static str,
    options: &[SettingOption],
    value: &str,
    state: fn(&mut V) -> &mut DropdownState,
    on_change: impl Fn(&mut V, String, &mut Window, &mut Context<V>) + Clone + 'static,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let selected = options.iter().position(|option| option.value == value);
    let label = selected
        .map(|index| options[index].label.clone())
        .unwrap_or_else(|| value.to_string());
    let (open, bounds) = {
        let dropdown = state(page);
        (dropdown.open, dropdown.trigger_bounds.clone())
    };
    let hover = p.raised_hover;
    let trigger = div()
        .w(px(SELECT_WIDTH))
        .max_w_full()
        .flex_shrink_0()
        .on_children_prepainted(capture_child_bounds(bounds, 0))
        .child(
            h_flex()
                .id(SharedString::from(format!("{id}-trigger")))
                .w_full()
                .h(px(CONTROL_HEIGHT))
                .px(px(12.0))
                .gap(px(6.0))
                .items_center()
                .justify_between()
                .rounded(px(MODAL_RADIUS_CONTROL))
                .border_1()
                .border_color(hsla(if open { p.focus_border } else { p.hairline }))
                .bg(hsla(p.raised))
                .text_size(px(14.0))
                .line_height(px(20.0))
                .text_color(hsla(p.foreground))
                .cursor_pointer()
                .hover(move |this| this.bg(hsla(hover)))
                .on_click(
                    cx.listener(move |page: &mut V, _: &ClickEvent, window, cx| {
                        toggle_dropdown(page, state, "Search...", selected, window, cx);
                    }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(label),
                )
                .child(settings_icon(icon::CHEVRON_DOWN, 16.0, p.muted).flex_shrink_0()),
        );
    let popup = if open {
        let query = state(page).query(cx);
        let kept: Vec<(String, String)> = options
            .iter()
            .filter(|option| modal_select_filter_matches(&query, &option.label))
            .map(|option| (option.value.clone(), option.label.clone()))
            .collect();
        let rows: Vec<DropdownRow> = kept
            .iter()
            .map(|(option_value, label)| {
                let mut row = DropdownRow::new(label.clone());
                row.checked = option_value == value;
                row
            })
            .collect();
        let dropdown = state(page);
        searchable_dropdown(
            p,
            format!("{id}-menu"),
            dropdown,
            state,
            &rows,
            "No matches found.",
            DropdownAlign::End,
            None,
            false,
            None,
            move |page: &mut V, index, window, cx| {
                if let Some((next, _)) = kept.get(index) {
                    on_change(page, next.clone(), window, cx);
                }
            },
            window,
            cx,
        )
    } else {
        None
    };
    div()
        .flex_shrink_0()
        .child(trigger)
        .children(popup)
        .into_any_element()
}

/// A setting row with a [`searchable_select`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn searchable_select_field<V: SettingsPage>(
    page: &mut V,
    p: &SettingsPalette,
    id: &'static str,
    spec: RowSpec,
    on_reset: Option<PageAction<V>>,
    options: &[SettingOption],
    value: &str,
    state: fn(&mut V) -> &mut DropdownState,
    on_change: impl Fn(&mut V, String, &mut Window, &mut Context<V>) + Clone + 'static,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let control = searchable_select(page, p, id, options, value, state, on_change, window, cx);
    setting_row(p, id, spec, on_reset, control, cx)
}
