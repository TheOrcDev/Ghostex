//! Git merge conflicts in a Markdown document: `<<<<<<<` … `=======` … `>>>>>>>` blocks get the
//! Docs page's Accept Current / Accept Incoming / Accept Both buttons in a row above their first line, in live
//! and source mode alike, as the former React Docs editor did.

use std::ops::Range;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    Pixels, SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use zorite_editor::{EditorEvent, EditorState};

use super::palette::DocsPalette;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MergeConflict {
    /// The `<<<<<<<` line's index.
    pub(crate) start_line: usize,
    /// The whole block, through the `>>>>>>>` line and its line break.
    pub(crate) block: Range<usize>,
    pub(crate) current_label: String,
    pub(crate) incoming_label: String,
    pub(crate) current_text: String,
    pub(crate) incoming_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConflictChoice {
    Current,
    Incoming,
    Both,
}

/// `parseMergeConflicts`: every complete conflict block, in document order.
pub(crate) fn parse_merge_conflicts(text: &str) -> Vec<MergeConflict> {
    // Line starts, and each line's end including its line break.
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    let line_count = starts.len();
    let line = |index: usize| -> &str {
        let start = starts[index];
        let end = starts.get(index + 1).map_or(text.len(), |next| next - 1);
        text[start..end].trim_end_matches('\r')
    };
    let end_with_break =
        |index: usize| -> usize { starts.get(index + 1).copied().unwrap_or(text.len()) };
    let mut conflicts = Vec::new();
    let mut index = 0;
    while index < line_count {
        let start_text = line(index);
        if !start_text.starts_with("<<<<<<<") {
            index += 1;
            continue;
        }
        let start_line = index;
        let mut base_line = None;
        let mut separator_line = None;
        let mut scan = start_line + 1;
        while scan < line_count {
            let text = line(scan);
            if separator_line.is_none() && base_line.is_none() && text.starts_with("|||||||") {
                base_line = Some(scan);
                scan += 1;
                continue;
            }
            if text.starts_with("=======") {
                separator_line = Some(scan);
                scan += 1;
                break;
            }
            scan += 1;
        }
        let Some(separator_line) = separator_line else {
            index = start_line + 1;
            continue;
        };
        let mut end_line = None;
        while scan < line_count {
            if line(scan).starts_with(">>>>>>>") {
                end_line = Some(scan);
                break;
            }
            scan += 1;
        }
        let Some(end_line) = end_line else {
            index = start_line + 1;
            continue;
        };
        let current_start = end_with_break(start_line);
        let current_end = starts[base_line.unwrap_or(separator_line)];
        let incoming_start = end_with_break(separator_line);
        let incoming_end = starts[end_line];
        conflicts.push(MergeConflict {
            start_line,
            block: starts[start_line]..end_with_break(end_line),
            current_label: start_text["<<<<<<<".len()..].trim().to_string(),
            incoming_label: line(end_line)[">>>>>>>".len()..].trim().to_string(),
            current_text: text[current_start..current_end.max(current_start)].to_string(),
            incoming_text: text[incoming_start..incoming_end.max(incoming_start)].to_string(),
        });
        index = end_line + 1;
    }
    conflicts
}

/// `applyConflictResolution`: the text that replaces the whole block.
pub(crate) fn resolution(text: &str, conflict: &MergeConflict, choice: ConflictChoice) -> String {
    match choice {
        ConflictChoice::Current => conflict.current_text.clone(),
        ConflictChoice::Incoming => conflict.incoming_text.clone(),
        ConflictChoice::Both => {
            let (current, incoming) = (&conflict.current_text, &conflict.incoming_text);
            if current.is_empty()
                || incoming.is_empty()
                || current.ends_with('\n')
                || incoming.starts_with(['\r', '\n'])
            {
                format!("{current}{incoming}")
            } else {
                let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
                format!("{current}{eol}{incoming}")
            }
        }
    }
}

/// Replaces a conflict block with the chosen side as one undoable edit, puts the caret at the
/// block's start, and reports the edit like a typed one.
pub(crate) fn resolve(
    editor: &Entity<EditorState>,
    start_line: usize,
    choice: ConflictChoice,
    cx: &mut App,
) {
    editor.update(cx, |editor, cx| {
        let text = editor.text().to_string();
        let Some(conflict) = parse_merge_conflicts(&text)
            .into_iter()
            .find(|conflict| conflict.start_line == start_line)
        else {
            return;
        };
        let insert = resolution(&text, &conflict, choice);
        editor.replace_range(conflict.block.clone(), &insert, cx);
        editor.set_cursor(conflict.block.start, cx);
        cx.emit(EditorEvent::Changed);
    });
}

/// The buttons for each conflict, in the row the editor keeps free above its `<<<<<<<` line, in the
/// editor's own coordinates (the rows the gutter uses). `None` when the document has no conflict.
pub(crate) fn render_conflict_actions(
    editor: &Entity<EditorState>,
    rows: &[(Pixels, Pixels)],
    p: &DocsPalette,
    cx: &App,
) -> Option<AnyElement> {
    let text = editor.read(cx).text();
    if !text.contains("<<<<<<<") {
        return None;
    }
    let conflicts = parse_merge_conflicts(text);
    if conflicts.is_empty() {
        return None;
    }
    // `--meo-color-base03` buttons with the editor's text, `base02` labels.
    let (button_bg, button_text, label) = if p.light {
        (
            gpui::rgb(0xd4d4d8),
            gpui::rgb(0x27272a),
            gpui::rgb(0x626269),
        )
    } else {
        (
            gpui::rgb(0x303030),
            gpui::rgb(0xd4d4d4),
            gpui::rgb(0x858585),
        )
    };
    let button_bg: gpui::Hsla = button_bg.into();
    let hover = gpui::black().blend(button_bg.opacity(0.92));
    let rows_layer = conflicts
        .into_iter()
        .filter_map(|conflict| {
            // The row the editor keeps free above the block's first line.
            let (top, _) = *rows.get(conflict.start_line)?;
            let height = px(zorite_editor::CONFLICT_ACTIONS_HEIGHT);
            let top = top - height;
            let labels = [
                (!conflict.current_label.is_empty())
                    .then(|| format!("Current: {},", conflict.current_label)),
                (!conflict.incoming_label.is_empty())
                    .then(|| format!("Incoming: {}", conflict.incoming_label)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("  ");
            let start_line = conflict.start_line;
            let button = |choice: ConflictChoice, text: &'static str| {
                let editor = editor.clone();
                div()
                    .id(SharedString::from(format!(
                        "docs-conflict-{start_line}-{text}"
                    )))
                    .flex_none()
                    .px(px(8.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(button_bg)
                    .text_color(button_text)
                    .text_size(px(11.0))
                    .line_height(px(11.0 * 1.35))
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .child(text)
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(move |_, window, cx| {
                        resolve(&editor, start_line, choice, cx);
                        editor.update(cx, |editor, cx| editor.focus(window, cx));
                    })
            };
            Some(
                div()
                    .absolute()
                    .top(top)
                    .h(height)
                    .left_0()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(button(ConflictChoice::Current, "Accept Current"))
                    .child(button(ConflictChoice::Incoming, "Accept Incoming"))
                    .child(button(ConflictChoice::Both, "Accept Both"))
                    .when(!labels.is_empty(), |row| {
                        row.child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .text_size(px(11.0))
                                .text_color(label)
                                .child(labels),
                        )
                    })
                    .into_any_element(),
            )
        })
        .collect::<Vec<_>>();
    (!rows_layer.is_empty()).then(|| {
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .children(rows_layer)
            .into_any_element()
    })
}
