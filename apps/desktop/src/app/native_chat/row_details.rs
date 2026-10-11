//! The text only an open row shows: a tool's arguments and result, a file card's diff.
//!
//! CDXC:SessionChat 2026-09-21 DECISION: User chose to stop sending text a collapsed row never shows. A row asks for its detail while it is drawn open (or, with File edit previews on, while a file card is on screen), and the host sends only those (`rowDetails` in gx-chat-core).

use super::state::NativeChatView;
use gpui::{Context, ListState, Window};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// One row of a transcript list: the session's own (`main`) or the subagent viewer's, with the
/// list's length when it was drawn, since a splice that changes the length moves the rows after it.
#[derive(Clone, Copy)]
struct RowRef {
    main: bool,
    index: usize,
    count: usize,
}

/// Which transcript row asked for each detail, and the rows drawn since the last sync.
#[derive(Default)]
pub(crate) struct DetailRows {
    drawing: Option<RowRef>,
    owners: HashMap<String, RowRef>,
    drawn_main: HashSet<usize>,
    drawn_subagent: HashSet<usize>,
}

impl DetailRows {
    /// A transcript row is being drawn: what it asks for until [`Self::end_row`] is its own.
    pub(super) fn start_row(&mut self, main: bool, index: usize, count: usize) {
        self.drawing = Some(RowRef { main, index, count });
        if main {
            self.drawn_main.insert(index);
        } else {
            self.drawn_subagent.insert(index);
        }
    }

    pub(super) fn end_row(&mut self) {
        self.drawing = None;
    }

    fn drawn(&self, main: bool) -> &HashSet<usize> {
        if main {
            &self.drawn_main
        } else {
            &self.drawn_subagent
        }
    }
}

impl NativeChatView {
    /// Ask for a row's detail for as long as the row keeps being drawn needing it; `Null` until it arrives.
    pub(super) fn row_detail(
        &mut self,
        key: &str,
        kind: &str,
        message_id: &str,
        index: u64,
    ) -> Value {
        if let Some(row) = self.detail_rows.drawing {
            self.detail_rows.owners.insert(key.to_string(), row);
        }
        self.detail_demand.insert(
            key.to_string(),
            json!({"key":key,"kind":kind,"messageId":message_id,"index":index}),
        );
        self.row_details[key].clone()
    }

    /// Whether `list`'s last layout drew every row it holds, so what those rows asked for is all they want.
    fn drew_every_held_row(list: &ListState, drawn: &HashSet<usize>) -> bool {
        let count = list.item_count();
        let held = list.last_measured_range();
        (held.start.min(count)..held.end.min(count)).all(|index| drawn.contains(&index))
    }

    /// Whether a detail sent earlier and not asked for since is no longer wanted: its row was drawn
    /// and did not ask (it closed), or every row the lists hold was drawn and none asked (it left).
    fn detail_released(&self, key: &str, rows: &DetailRows, every_held_row_drawn: bool) -> bool {
        if every_held_row_drawn {
            return true;
        }
        rows.owners.get(key).is_some_and(|row| {
            let list = if row.main {
                &self.list
            } else {
                &self.subagent_list
            };
            row.count == list.item_count() && rows.drawn(row.main).contains(&row.index)
        })
    }

    /// After this frame's rows are drawn, tell the host which details they asked for, if that changed.
    ///
    /// CDXC:SessionChat 2026-10-01 WHY:
    /// Not being drawn is not the same as closing. gpui's list keeps the rows in its overdraw (the band just above and below the viewport) at the size they were last drawn and draws them again only once they are remeasured, so an open row there asked for its detail on the frame after a remeasure and was silent on the next. Taking that silence for "closed" dropped the detail, the host's reply remeasured the list, the row was drawn again without it and asked again: its height flipped on every sync, the transcript's height with it, and the scrollbar thumb jumped between two sizes and places for as long as the row sat just off screen. So a detail goes only when its own row is drawn without asking, or on a frame that drew every row the list holds (the one after any remeasure), which is when a row that scrolled away lets go.
    pub(super) fn schedule_row_detail_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.detail_sync_scheduled {
            return;
        }
        self.detail_sync_scheduled = true;
        let chat = cx.weak_entity();
        window.on_next_frame(move |_, cx| {
            let _ = chat.update(cx, |chat, cx| {
                chat.detail_sync_scheduled = false;
                let mut demand = std::mem::take(&mut chat.detail_demand);
                let mut rows = std::mem::take(&mut chat.detail_rows);
                let every_held_row_drawn = Self::drew_every_held_row(&chat.list, &rows.drawn_main)
                    && (!chat.snapshot["subagent"].is_object()
                        || Self::drew_every_held_row(&chat.subagent_list, &rows.drawn_subagent));
                for (key, row) in &chat.detail_sent {
                    if !demand.contains_key(key)
                        && !chat.detail_released(key, &rows, every_held_row_drawn)
                    {
                        demand.insert(key.clone(), row.clone());
                    }
                }
                rows.owners.retain(|key, _| demand.contains_key(key));
                chat.detail_rows = DetailRows {
                    owners: rows.owners,
                    ..DetailRows::default()
                };
                if demand == chat.detail_sent {
                    return;
                }
                let open: Vec<Value> = demand.values().cloned().collect();
                chat.detail_sent = demand;
                chat.invoke(json!({"type":"rowDetails","open":open}), cx);
            });
        });
    }
}
