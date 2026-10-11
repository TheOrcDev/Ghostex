//! The demo root view: a chat-shaped `gpui::list` that proves the embedding before the real
//! transcript arrives. It uses the list the way `native_chat` does (`ListAlignment::Top`, 400 px
//! overdraw, `FollowMode::Tail`), variable-height rows, tap-to-expand with a remeasure, and
//! appends that follow the tail only while the reader is at the bottom.
//!
//! Commands: `appendRow {text}`, `appendRows {count}`, `scrollToEnd`, `scrollToTop`,
//! `reset {count}`, `state`. Events: `rowTapped {index, id, expanded}`, `appended {rows,
//! following}`, `state {...}`.

mod rows;

use gpui::{
    AnyElement, App, AppContext as _, Context, FollowMode, FontWeight, InteractiveElement,
    IntoElement, ListAlignment, ListOffset, ListState, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, list, prelude::FluentBuilder as _, px, rgb,
};
use serde_json::{Value, json};

use crate::{EventSink, HostConfig, RootContent, fonts::UI_FAMILY};
use rows::{DemoRow, Role};

const DEFAULT_ROWS: u64 = 300;

pub(crate) fn build(
    _window: &mut Window,
    cx: &mut App,
    config: &HostConfig,
    events: EventSink,
) -> RootContent {
    let count = config.u64("demoRows").unwrap_or(DEFAULT_ROWS);
    let view = cx.new(|_| DemoTranscript::new(count, events));
    let handler_view = view.clone();
    RootContent {
        view: view.into(),
        on_command: Box::new(move |command, _window, cx| {
            handler_view.update(cx, |view, cx| view.command(command, cx))
        }),
    }
}

pub(crate) struct DemoTranscript {
    rows: Vec<DemoRow>,
    next_id: u64,
    list: ListState,
    events: EventSink,
}

impl DemoTranscript {
    fn new(count: u64, events: EventSink) -> Self {
        let rows: Vec<DemoRow> = (0..count).map(rows::generated).collect();
        let list = ListState::new(rows.len(), ListAlignment::Top, px(400.0));
        list.set_follow_mode(FollowMode::Tail);
        Self {
            next_id: count,
            rows,
            list,
            events,
        }
    }

    fn command(&mut self, command: &Value, cx: &mut Context<Self>) -> bool {
        match command
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "appendRow" => {
                let text = command
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    let id = self.take_id();
                    self.append(vec![rows::typed(id, text)], cx);
                }
            }
            "appendRows" => {
                let count = command
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .min(10_000);
                let rows = (0..count)
                    .map(|_| {
                        let id = self.take_id();
                        rows::generated(id)
                    })
                    .collect();
                self.append(rows, cx);
            }
            "scrollToEnd" => {
                self.list.set_follow_mode(FollowMode::Tail);
                cx.notify();
            }
            "scrollToTop" => {
                self.list.scroll_to(ListOffset::default());
                cx.notify();
            }
            "reset" => {
                let count = command
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(DEFAULT_ROWS)
                    .min(10_000);
                self.rows = (0..count).map(rows::generated).collect();
                self.next_id = count;
                self.list.reset(self.rows.len());
                self.list.set_follow_mode(FollowMode::Tail);
                cx.notify();
            }
            "state" => self.emit_state(),
            _ => return false,
        }
        true
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// New rows go at the end. `FollowMode::Tail` keeps the list pinned to them only while the
    /// reader is at the bottom; someone reading older rows stays where they are.
    fn append(&mut self, new_rows: Vec<DemoRow>, cx: &mut Context<Self>) {
        let start = self.rows.len();
        let count = new_rows.len();
        self.rows.extend(new_rows);
        self.list.splice(start..start, count);
        cx.notify();
        self.events.emit(json!({
            "type": "appended",
            "rows": self.rows.len(),
            "following": self.list.is_following_tail(),
        }));
    }

    fn toggle(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get_mut(index) else {
            return;
        };
        row.expanded = !row.expanded;
        let (id, expanded) = (row.id, row.expanded);
        self.list.remeasure_items(index..index + 1);
        cx.notify();
        log::info!("row {index} tapped, expanded={expanded}");
        self.events.emit(json!({
            "type": "rowTapped",
            "index": index,
            "id": id,
            "expanded": expanded,
        }));
    }

    fn emit_state(&self) {
        let top = self.list.logical_scroll_top();
        self.events.emit(json!({
            "type": "state",
            "rows": self.rows.len(),
            "following": self.list.is_following_tail(),
            "scrollTopItem": top.item_ix,
            "scrollTopOffset": f32::from(top.offset_in_item),
            "expanded": self.rows.iter().filter(|row| row.expanded).count(),
        }));
    }

    fn render_row(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(index) else {
            return div().into_any_element();
        };
        let role_color = match row.role {
            Role::User => rgb(0x8ab4f8),
            Role::Assistant => rgb(0xc4a7e7),
            Role::Tool => rgb(0x9ccfa4),
        };
        // DM Sans has no arrow glyphs, and the demo only draws with the family it registers.
        let chevron = if row.expanded { "Hide" } else { "Show" };
        let body: SharedString = row.body.clone();
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .text_size(px(12.0))
            .child(
                div()
                    .text_color(role_color)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(row.role.label()),
            )
            .child(div().text_color(rgb(0x6c6c6c)).child(format!("#{index}")))
            .child(div().flex_1())
            .child(div().text_color(rgb(0x6c6c6c)).child(chevron));
        let body = match row.role {
            Role::User => div()
                .mt(px(4.0))
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(14.0))
                .bg(rgb(0x2b2d31))
                .text_color(rgb(0xeeeeee))
                .child(body),
            Role::Tool => div()
                .mt(px(4.0))
                .text_size(px(13.0))
                .text_color(rgb(0xa0a0a0))
                .child(body),
            Role::Assistant => div().mt(px(4.0)).text_color(rgb(0xe3e3e3)).child(body),
        };
        let detail = row.detail.clone();
        div()
            .id(("demo-row", row.id))
            .w_full()
            .px(px(14.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(rgb(0x232323))
            .child(header)
            .child(body)
            .when(row.expanded, |this| {
                this.child(
                    div()
                        .mt(px(8.0))
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(rgb(0x222428))
                        .text_size(px(13.0))
                        .line_height(px(19.0))
                        .text_color(rgb(0xb8b8b8))
                        .child(detail),
                )
            })
            .on_click(cx.listener(move |this, _event, _window, cx| this.toggle(index, cx)))
            .into_any_element()
    }
}

impl Render for DemoTranscript {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x181818))
            .font_family(UI_FAMILY)
            .text_size(px(15.0))
            .line_height(px(22.0))
            .text_color(rgb(0xe3e3e3))
            .child(
                list(
                    self.list.clone(),
                    cx.processor(|this, index: usize, _window, cx| this.render_row(index, cx)),
                )
                .size_full(),
            )
    }
}
