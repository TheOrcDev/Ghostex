//! `CommandIconPicker` (packages/core-ui/command-icon-picker.tsx (deleted 2026-10-01)), the labelled field of the
//! action editor: an uppercase "ICON" label over the `.group-title-input` trigger (the chosen
//! glyph, its label and a chevron) and the searchable popover of the 59 command icons
//! (`SIDEBAR_COMMAND_ICON_IDS`, filtered by label with cmdk's scorer). The glyphs are the Tabler
//! icons `SidebarCommandIconGlyph` draws (filled where packages/core-ui/sidebar-command-icon.tsx (deleted 2026-10-01)
//! uses the filled variant).
//!
//! CDXC:Icons 2026-06-16-07:48 SEE-ALSO: action glyphs take the surrounding foreground colour, never a per-action colour (packages/core-ui/command-icon-picker.tsx (deleted 2026-10-01)).
use super::super::super::native_modal_kit::*;
use super::super::super::space_editor_modal::SPACE_EDITOR_ICONS;
use super::super::palette::SettingsPalette;
use super::searchable_list::{
    SearchableList, SearchableListKey, SearchablePopoverSpec, filter_by_score, searchable_popover,
    searchable_row,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement as _, Rgba, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    div, px, rgb,
};
use gpui_component::{h_flex, v_flex};

/// `DEFAULT_SIDEBAR_COMMAND_ICON`.
pub(crate) const DEFAULT_COMMAND_ICON: &str = "playerPlay";
const ICON_CHEVRON_DOWN: &str = "modals/settings/chevron-down.svg";

/// The glyph file of each `SPACE_EDITOR_ICONS` entry, in the same order (`ICON_COMPONENT_BY_ID`).
const COMMAND_ICON_ASSETS: [&str; 59] = [
    "modals/space-editor/player-play-filled.svg",
    "modals/space-editor/api.svg",
    "modals/space-editor/archive-filled.svg",
    "modals/space-editor/bell-filled.svg",
    "modals/space-editor/bolt-filled.svg",
    "modals/space-editor/book-filled.svg",
    "modals/space-editor/brain.svg",
    "modals/space-editor/braces.svg",
    "modals/space-editor/brand-docker.svg",
    "modals/space-editor/brand-github-filled.svg",
    "modals/space-editor/brand-python.svg",
    "modals/space-editor/brand-react.svg",
    "modals/space-editor/brand-vscode.svg",
    "modals/space-editor/bug-filled.svg",
    "modals/space-editor/chart-bar.svg",
    "modals/space-editor/cloud-filled.svg",
    "modals/space-editor/checklist.svg",
    "modals/space-editor/clock-filled.svg",
    "modals/space-editor/code.svg",
    "modals/space-editor/command.svg",
    "modals/space-editor/cpu.svg",
    "modals/space-editor/database-filled.svg",
    "modals/space-editor/device-desktop-filled.svg",
    "modals/space-editor/device-laptop.svg",
    "modals/space-editor/download-filled.svg",
    "modals/space-editor/file-code-filled.svg",
    "modals/space-editor/file-diff-filled.svg",
    "modals/space-editor/file-search.svg",
    "modals/space-editor/file-text-filled.svg",
    "modals/space-editor/flask-filled.svg",
    "modals/space-editor/folder-filled.svg",
    "modals/space-editor/folder-open-filled.svg",
    "modals/space-editor/git-branch.svg",
    "modals/space-editor/git-commit.svg",
    "modals/space-editor/git-merge.svg",
    "modals/space-editor/git-pull-request.svg",
    "modals/space-editor/key-filled.svg",
    "modals/space-editor/layout-dashboard-filled.svg",
    "modals/space-editor/link.svg",
    "modals/space-editor/lock-filled.svg",
    "modals/space-editor/message-circle-filled.svg",
    "modals/space-editor/package.svg",
    "modals/space-editor/pencil-code.svg",
    "modals/space-editor/refresh.svg",
    "modals/space-editor/robot.svg",
    "modals/space-editor/route.svg",
    "modals/space-editor/rocket.svg",
    "modals/space-editor/search-filled.svg",
    "modals/space-editor/server.svg",
    "modals/space-editor/settings-filled.svg",
    "modals/space-editor/shield-search.svg",
    "modals/space-editor/sparkles-filled.svg",
    "modals/space-editor/stack-filled.svg",
    "modals/space-editor/terminal-2.svg",
    "modals/space-editor/test-pipe.svg",
    "modals/space-editor/tool.svg",
    "modals/space-editor/upload.svg",
    "modals/space-editor/wand.svg",
    "modals/space-editor/world.svg",
];

fn icon_index(id: &str) -> usize {
    SPACE_EDITOR_ICONS
        .iter()
        .position(|icon| icon.id == id)
        .or_else(|| {
            SPACE_EDITOR_ICONS
                .iter()
                .position(|icon| icon.id == DEFAULT_COMMAND_ICON)
        })
        .unwrap_or(0)
}

/// `isSidebarCommandIcon`.
pub(crate) fn is_command_icon(id: &str) -> bool {
    SPACE_EDITOR_ICONS.iter().any(|icon| icon.id == id)
}

/// The glyph asset `SidebarCommandIconGlyph` draws for a command icon id (the default Play for
/// an unknown one).
pub(crate) fn command_icon_glyph(id: &str) -> &'static str {
    COMMAND_ICON_ASSETS[icon_index(id)]
}

/// `getSidebarCommandIconLabel`.
pub(crate) fn command_icon_label(id: &str) -> &'static str {
    SPACE_EDITOR_ICONS[icon_index(id)].label
}

/// A command icon glyph at `size` in `color`.
pub(crate) fn command_icon(id: &str, size: f32, color: Rgba) -> AnyElement {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .size(px(size))
        .child(modal_icon(command_icon_glyph(id), size, color))
        .into_any_element()
}

/// `--app-muted` (#717171 in the light theme) and `--app-card-active` of the picker CSS.
fn app_tones(p: &SettingsPalette) -> (Rgba, Rgba, Rgba) {
    if p.light {
        (rgb(0x717171), rgb(0xefefef), rgb(0xf4f4f5))
    } else {
        (p.muted, rgb(0x2a2a2a), rgb(0x0e0e0e))
    }
}

/// The page's picker state.
pub(crate) type SearchableListAccess<V> = fn(&mut V) -> &mut SearchableList;

/// The labelled icon field with its popover. `access` reaches the page's picker state;
/// `on_choose` runs with the chosen icon id after the popover closed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn command_icon_picker_field<V: 'static>(
    page: &mut V,
    p: &SettingsPalette,
    id: &str,
    access: SearchableListAccess<V>,
    selected: &str,
    on_choose: impl Fn(&mut V, &'static str, &mut Window, &mut Context<V>) + Clone + 'static,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let (app_muted, card_active, app_background) = app_tones(p);
    let list = access(page);
    list.sync_query(cx);
    let query = list.query(cx);
    let labels: Vec<String> = SPACE_EDITOR_ICONS
        .iter()
        .map(|icon| icon.label.to_string())
        .collect();
    let results = filter_by_score(&labels, &query);
    let (highlight, focus, trigger_bounds) =
        (list.highlight, list.focus.clone(), list.trigger.clone());
    let focused = focus.is_focused(window);
    let selected_index = icon_index(selected);
    let chevron = css_mix(app_muted, 0.84, p.foreground);
    let trigger = h_flex()
        .id(SharedString::from(format!("{id}-trigger")))
        .track_focus(&focus)
        .w_full()
        .px(px(8.1))
        .py(px(5.4))
        .gap(px(10.0))
        .items_center()
        .justify_between()
        .border_1()
        .border_color(hsla(if focused { p.focus_border } else { p.hairline }))
        .bg(hsla(css_fade(card_active, 0.92)))
        .text_size(px(14.0))
        .line_height(px(14.0))
        .text_color(hsla(p.foreground))
        .cursor_pointer()
        .on_click(
            cx.listener(move |page: &mut V, _: &ClickEvent, window, cx| {
                let list = access(page);
                if list.open {
                    list.close(window, cx);
                } else {
                    list.open(false, window, cx);
                }
                cx.notify();
            }),
        )
        .child(
            h_flex()
                .min_w_0()
                .items_center()
                .gap(px(8.0))
                .child(command_icon(
                    SPACE_EDITOR_ICONS[selected_index].id,
                    16.0,
                    p.foreground,
                ))
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(SPACE_EDITOR_ICONS[selected_index].label),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .child(modal_icon(ICON_CHEVRON_DOWN, 16.0, chevron)),
        );
    let rows: Vec<AnyElement> = results
        .iter()
        .enumerate()
        .map(|(position, index)| {
            let index = *index;
            let icon = &SPACE_EDITOR_ICONS[index];
            let checked = index == selected_index;
            let on_choose = on_choose.clone();
            let icon_id = icon.id;
            let row = searchable_row(
                p,
                (SharedString::from(format!("{id}-option")), index),
                32.0,
                6.0,
                8.0,
                position == highlight,
                checked,
                vec![
                    command_icon(icon.id, 16.0, p.foreground),
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(icon.label)
                        .into_any_element(),
                ],
                move |page: &mut V, window, cx| {
                    access(page).close(window, cx);
                    cx.notify();
                    on_choose(page, icon_id, window, cx);
                },
                cx,
            );
            // The chosen row keeps `--app-context-menu-hover-background` under the highlight.
            if checked && position != highlight {
                div()
                    .rounded(px(6.0))
                    .bg(hsla(if p.light {
                        rgb(0xefefef)
                    } else {
                        rgb(0x202020)
                    }))
                    .child(row)
                    .into_any_element()
            } else {
                row
            }
        })
        .collect();
    let spec = SearchablePopoverSpec {
        id: SharedString::from(format!("{id}-popover")),
        background: css_mix(card_active, 0.96, app_background),
        border: if p.light {
            modal_rgba(0x000000, 0.14)
        } else {
            modal_rgba(0xffffff, 0.12)
        },
        shadow: true,
        align_end: false,
        side_offset: 4.0,
        list_max_height: 288.0,
        heading: None,
        empty_text: "No matching icons".into(),
        empty_compact: true,
        row_height: 32.0,
    };
    let popover = searchable_popover(
        p,
        access(page),
        spec,
        rows,
        move |page: &mut V, window, cx| {
            access(page).close(window, cx);
            cx.notify();
        },
        move |page: &mut V, window, cx| {
            let list = access(page);
            list.search.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
            cx.notify();
        },
        window,
        cx,
    );
    let key_results = results.clone();
    let key_choose = on_choose.clone();
    v_flex()
        .w_full()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .line_height(px(15.7))
                .text_color(hsla(app_muted))
                .child("ICON"),
        )
        .child(
            div()
                .w_full()
                .on_key_down(
                    cx.listener(move |page: &mut V, event: &KeyDownEvent, window, cx| {
                        let list = access(page);
                        if !list.open {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | "down")
                                && list.focus.is_focused(window)
                            {
                                list.open(false, window, cx);
                                cx.stop_propagation();
                                cx.notify();
                            }
                            return;
                        }
                        match list.handle_key(event, key_results.len(), window, cx) {
                            SearchableListKey::Choose(position) => {
                                list.close(window, cx);
                                if let Some(index) = key_results.get(position) {
                                    key_choose(page, SPACE_EDITOR_ICONS[*index].id, window, cx);
                                }
                            }
                            SearchableListKey::Close => list.close(window, cx),
                            SearchableListKey::Moved => {}
                            SearchableListKey::Ignored => return,
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .on_children_prepainted(capture_child_bounds(trigger_bounds, 0))
                .child(trigger)
                .when_some(popover, |this, popover| this.child(popover)),
        )
        .into_any_element()
}
