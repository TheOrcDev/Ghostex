//! `SearchableDropdownContent` + `Command` (packages/components/ui/searchable-dropdown.tsx and
//! command.tsx) as Settings uses them: a popover anchored to its trigger with a pinned search row
//! (an 8px-padded 32px input group, the search glyph on its right, a clear button once a query is
//! typed, a hairline under it), an optional group heading, and a scrolling list of 6px-rounded
//! rows whose hovered or keyboard-highlighted row takes the foreground/8 fill and whose checked
//! row carries a check on its right. Filtering is cmdk's scorer (`command_score`), best first.
//!
//! The command icon picker (fields/icon_picker.rs) and the Projects page's project selector are
//! built on it. The page owns a [`SearchableList`] and wires its keys and clicks; this module only
//! draws.
use super::super::super::native_modal_kit::*;
use super::super::super::space_editor_modal::command_score;
use super::super::palette::SettingsPalette;
use gpui::Focusable as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Anchor, AnyElement, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    FontWeight, InteractiveElement as _, IntoElement, KeyDownEvent, MouseDownEvent,
    ParentElement as _, Pixels, Rgba, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, anchored, deferred, div, point, px,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Sizable as _, Size as ComponentSize, h_flex, v_flex};
use std::cell::Cell;
use std::rc::Rc;

const ICON_SEARCH: &str = "modals/settings/search.svg";
const ICON_CLEAR: &str = "modals/settings/x.svg";
const ICON_CHECK: &str = "modals/settings/check.svg";
/// The search row: 8px padding around the 32px group, and its hairline.
const SEARCH_BLOCK_HEIGHT: f32 = 8.0 + 32.0 + 8.0 + 1.0;
/// Base UI's collision padding.
const WINDOW_MARGIN: f32 = 5.0;

/// What a key did while the popover was open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchableListKey {
    /// Enter on the highlighted row (its position in the filtered list).
    Choose(usize),
    /// Escape.
    Close,
    /// An arrow moved the highlight.
    Moved,
    Ignored,
}

/// The state of one searchable popover.
pub(crate) struct SearchableList {
    pub(crate) open: bool,
    pub(crate) search: Entity<InputState>,
    /// The keyboard-highlighted row, a position in the filtered list.
    pub(crate) highlight: usize,
    pub(crate) scroll: ScrollHandle,
    /// The trigger's window bounds, captured each frame (`capture_child_bounds`).
    pub(crate) trigger: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The trigger's focus: Escape and the arrows reach the popover from here while the
    /// search field is not focused.
    pub(crate) focus: FocusHandle,
    last_query: String,
    _subscription: Subscription,
}

impl SearchableList {
    pub(crate) fn new<V: 'static>(
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Self {
        let placeholder = placeholder.to_string();
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let subscription = cx.subscribe_in(
            &search,
            window,
            |_page: &mut V, _input, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );
        Self {
            open: false,
            search,
            highlight: 0,
            scroll: ScrollHandle::new(),
            trigger: Rc::new(Cell::new(None)),
            focus: cx.focus_handle().tab_stop(true),
            last_query: String::new(),
            _subscription: subscription,
        }
    }

    pub(crate) fn query(&self, cx: &gpui::App) -> String {
        self.search.read(cx).value().to_string()
    }

    /// Opens with an empty query and the first row highlighted. `focus_search` puts the caret in
    /// the search field (the project selector); the icon picker keeps focus on its trigger
    /// (`onOpenAutoFocus` prevented).
    pub(crate) fn open<V>(&mut self, focus_search: bool, window: &mut Window, cx: &mut Context<V>) {
        self.open = true;
        self.highlight = 0;
        self.last_query.clear();
        self.scroll.scroll_to_item(0);
        self.search.update(cx, |input, cx| {
            input.set_value("", window, cx);
            if focus_search {
                input.focus(window, cx);
            }
        });
        if !focus_search {
            self.focus.focus(window, cx);
        }
    }

    /// Closes and hands focus back to the trigger.
    pub(crate) fn close<V>(&mut self, window: &mut Window, cx: &mut Context<V>) {
        self.open = false;
        self.focus.focus(window, cx);
    }

    /// A new query puts the highlight back on the first row (cmdk's behaviour).
    pub(crate) fn sync_query(&mut self, cx: &gpui::App) {
        let query = self.query(cx);
        if query != self.last_query {
            self.last_query = query;
            self.highlight = 0;
            self.scroll.scroll_to_item(0);
        }
    }

    /// The popover's keys: arrows move the highlight over `count` rows, Enter chooses it, Escape
    /// closes (after clearing a typed query, as `CommandInput` does).
    pub(crate) fn handle_key<V>(
        &mut self,
        event: &KeyDownEvent,
        count: usize,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> SearchableListKey {
        if !self.open {
            return SearchableListKey::Ignored;
        }
        match event.keystroke.key.as_str() {
            "down" if count > 0 => {
                self.highlight = (self.highlight + 1).min(count - 1);
                self.scroll.scroll_to_item(self.highlight);
                SearchableListKey::Moved
            }
            "up" if count > 0 => {
                self.highlight = self.highlight.saturating_sub(1);
                self.scroll.scroll_to_item(self.highlight);
                SearchableListKey::Moved
            }
            "enter" if count > 0 => SearchableListKey::Choose(self.highlight.min(count - 1)),
            "escape" => {
                let search_focused = self.search.read(cx).focus_handle(cx).is_focused(window);
                if search_focused && !self.query(cx).is_empty() {
                    self.search
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    SearchableListKey::Moved
                } else {
                    SearchableListKey::Close
                }
            }
            _ => SearchableListKey::Ignored,
        }
    }
}

/// cmdk's filter: every item while the query is empty, otherwise the items whose value scores
/// above zero, best score first (stable).
pub(crate) fn filter_by_score(values: &[String], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..values.len()).collect();
    }
    let mut scored: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .map(|(index, value)| (index, command_score(value, query)))
        .filter(|(_, score)| *score > 0.0)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(index, _)| index).collect()
}

/// The popover's surface and geometry.
pub(crate) struct SearchablePopoverSpec {
    pub(crate) id: SharedString,
    pub(crate) background: Rgba,
    pub(crate) border: Rgba,
    /// `.searchable-dropdown`'s `0 12px 28px rgba(0, 0, 0, 0.35)`.
    pub(crate) shadow: bool,
    /// Right edges aligned (`align='end'`) instead of left edges.
    pub(crate) align_end: bool,
    /// `sideOffset`.
    pub(crate) side_offset: f32,
    /// The list's height cap (`max-height` of the command list).
    pub(crate) list_max_height: f32,
    /// A `CommandGroup` heading above the rows.
    pub(crate) heading: Option<SharedString>,
    pub(crate) empty_text: SharedString,
    /// `CommandEmpty`'s look: the icon picker's 12px muted note in 10px padding, or the stock
    /// `py-6 text-center text-sm`.
    pub(crate) empty_compact: bool,
    /// The natural height of one row, for placing the popover above the trigger when there is
    /// no room below.
    pub(crate) row_height: f32,
}

/// `--input` and the command input group's fill.
fn input_colors(p: &SettingsPalette) -> (Rgba, Rgba, Rgba) {
    let border = if p.light {
        modal_rgba(0x000000, 0.16)
    } else {
        modal_rgba(0xffffff, 0.15)
    };
    let fill = rgba_of(border, border.a * 0.30);
    let focused = if p.light {
        modal_rgba(0x000000, 0.16)
    } else {
        gpui::rgb(0x737373)
    };
    (border, fill, focused)
}

/// The popover's search row (`CommandInput` with its clear button).
fn search_row<V: 'static>(
    p: &SettingsPalette,
    id: &SharedString,
    list: &SearchableList,
    window: &Window,
    cx: &mut Context<V>,
    on_clear: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> AnyElement {
    let has_query = !list.query(cx).is_empty();
    let (input_border, input_fill, focused_border) = input_colors(p);
    let row_border = if p.light {
        modal_rgba(0x000000, 0.14)
    } else {
        modal_rgba(0xffffff, 0.12)
    };
    let focused = list.search.read(cx).focus_handle(cx).is_focused(window);
    let muted = p.muted;
    let foreground = p.foreground;
    div()
        .w_full()
        .p(px(8.0))
        .border_b_1()
        .border_color(hsla(row_border))
        .child(
            h_flex()
                .w_full()
                .h(px(32.0))
                .items_center()
                .rounded(px(6.0))
                .border_1()
                .border_color(hsla(if focused {
                    focused_border
                } else {
                    input_border
                }))
                .bg(hsla(input_fill))
                .child(
                    div().flex_1().min_w_0().pl(px(10.0)).child(
                        Input::new(&list.search)
                            .with_size(ComponentSize::Small)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .w_full()
                            .px(px(0.0))
                            .py(px(0.0))
                            .text_size(px(13.0))
                            .text_color(hsla(p.foreground)),
                    ),
                )
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .items_center()
                        .pr(px(if has_query { 8.0 } else { 12.0 }))
                        .child(if has_query {
                            div()
                                .id(SharedString::from(format!("{id}-clear")))
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(24.0))
                                .cursor_pointer()
                                .text_color(hsla(muted))
                                .hover(move |this| this.text_color(hsla(foreground)))
                                .on_click(cx.listener(move |page, _: &ClickEvent, window, cx| {
                                    on_clear(page, window, cx);
                                }))
                                .child(modal_icon(ICON_CLEAR, 16.0, muted))
                                .into_any_element()
                        } else {
                            div()
                                .opacity(0.5)
                                .child(modal_icon(ICON_SEARCH, 16.0, muted))
                                .into_any_element()
                        }),
                ),
        )
        .into_any_element()
}

/// One row of the list: 6px-rounded, the foreground/8 fill when hovered or highlighted, and the
/// check on its right when it is the chosen value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn searchable_row<V: 'static>(
    p: &SettingsPalette,
    id: impl Into<gpui::ElementId>,
    min_height: f32,
    padding_y: f32,
    gap: f32,
    highlighted: bool,
    checked: bool,
    content: Vec<AnyElement>,
    on_click: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> AnyElement {
    let fill = hsla(p.foreground_alpha(0.08));
    h_flex()
        .id(id)
        .w_full()
        .min_h(px(min_height))
        .px(px(10.0))
        .py(px(padding_y))
        .gap(px(gap))
        .items_center()
        .rounded(px(6.0))
        .text_size(px(13.0))
        .line_height(px(20.0))
        .text_color(hsla(p.foreground))
        .cursor_default()
        .when(highlighted, |this| this.bg(fill))
        .hover(move |this| this.bg(fill))
        .on_click(cx.listener(move |page, _: &ClickEvent, window, cx| {
            on_click(page, window, cx);
        }))
        .children(content)
        .when(checked, |this| {
            this.child(div().ml_auto().flex_shrink_0().child(modal_icon(
                ICON_CHECK,
                16.0,
                p.foreground,
            )))
        })
        .into_any_element()
}

/// The popover, anchored under its trigger (above it when the room below is too short), as wide
/// as the trigger. `rows` are the filtered rows in order; `on_dismiss` runs for a click outside
/// the popover and its trigger; `on_clear` clears the query from the clear button.
#[allow(clippy::too_many_arguments)]
pub(crate) fn searchable_popover<V: 'static>(
    p: &SettingsPalette,
    list: &SearchableList,
    spec: SearchablePopoverSpec,
    rows: Vec<AnyElement>,
    on_dismiss: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    on_clear: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    window: &mut Window,
    cx: &mut Context<V>,
) -> Option<AnyElement> {
    if !list.open {
        return None;
    }
    let trigger = list.trigger.get()?;
    let viewport = window.viewport_size();
    let heading_height = if spec.heading.is_some() { 32.0 } else { 0.0 };
    let empty_height = if spec.empty_compact {
        10.0 + 17.4 + 10.0
    } else {
        24.0 + 20.0 + 24.0
    };
    let list_natural = if rows.is_empty() {
        empty_height + 8.0
    } else {
        heading_height + rows.len() as f32 * spec.row_height + 8.0
    };
    let natural = SEARCH_BLOCK_HEIGHT + list_natural + 2.0;
    let below = f32::from(viewport.height)
        - f32::from(trigger.origin.y + trigger.size.height)
        - spec.side_offset
        - WINDOW_MARGIN;
    let above = f32::from(trigger.origin.y) - spec.side_offset - WINDOW_MARGIN;
    let (available, flip_above) = if natural <= below || below >= above {
        (below, false)
    } else {
        (above, true)
    };
    let list_height = list_natural
        .min(spec.list_max_height)
        .min((available - SEARCH_BLOCK_HEIGHT - 2.0).max(spec.row_height));
    let empty_color = css_mix(p.muted, 0.88, p.foreground);
    let body = if rows.is_empty() && spec.empty_compact {
        div()
            .w_full()
            .p(px(10.0))
            .text_size(px(12.0))
            .line_height(px(17.4))
            .text_center()
            .text_color(hsla(empty_color))
            .child(spec.empty_text.clone())
            .into_any_element()
    } else if rows.is_empty() {
        div()
            .w_full()
            .py(px(24.0))
            .text_size(px(14.0))
            .line_height(px(20.0))
            .text_center()
            .text_color(hsla(p.foreground))
            .child(spec.empty_text.clone())
            .into_any_element()
    } else {
        v_flex()
            .id(SharedString::from(format!("{}-list", spec.id)))
            .w_full()
            .max_h(px(list_height))
            .p(px(4.0))
            .overflow_y_scroll()
            .track_scroll(&list.scroll)
            .children(spec.heading.clone().map(|heading| {
                div()
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(hsla(p.muted))
                    .child(heading)
            }))
            .children(rows)
            .into_any_element()
    };
    let id = spec.id.clone();
    let search = search_row(p, &id, list, window, cx, on_clear);
    let trigger_bounds = trigger;
    let x = if spec.align_end {
        trigger.origin.x + trigger.size.width
    } else {
        trigger.origin.x
    };
    let (y, anchor) = if flip_above {
        (
            trigger.origin.y - px(spec.side_offset),
            if spec.align_end {
                Anchor::BottomRight
            } else {
                Anchor::BottomLeft
            },
        )
    } else {
        (
            trigger.origin.y + trigger.size.height + px(spec.side_offset),
            if spec.align_end {
                Anchor::TopRight
            } else {
                Anchor::TopLeft
            },
        )
    };
    let panel = v_flex()
        .id(id)
        .occlude()
        .w(trigger.size.width)
        .overflow_hidden()
        .rounded(px(8.0))
        .border_1()
        .border_color(hsla(spec.border))
        .bg(hsla(spec.background))
        .when(spec.shadow, |this| {
            this.shadow(vec![gpui::BoxShadow {
                color: hsla(modal_rgba(0x000000, 0.35)),
                offset: point(px(0.0), px(12.0)),
                blur_radius: px(28.0),
                spread_radius: px(0.0),
                inset: false,
            }])
        })
        .font_family(MODAL_UI_FONT)
        .on_mouse_down_out(
            cx.listener(move |page, event: &MouseDownEvent, window, cx| {
                if trigger_bounds.contains(&event.position) {
                    return;
                }
                on_dismiss(page, window, cx);
            }),
        )
        .child(search)
        .child(body);
    Some(
        deferred(
            anchored()
                .position(point(x, y))
                .anchor(anchor)
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(panel),
        )
        .with_priority(1)
        .into_any_element(),
    )
}
