//! Marked sections: the parts of an agent's reply that start with a Marked handovers marker
//! (❓ Needs you, ✅ Done, ⚠️ Read this, 🔄 In progress, 🧪 Untested, 🚀 Shipped, 🛑 Blocked),
//! tagged with their kind so every chat draws them in that kind's colour, and the summary of the
//! ones that ask something of the reader.
//!
//! The pass runs over the native Markdown (`native_markdown.rs`) and wraps each section in marks,
//! the way an alert or a table is wrapped:
//!
//!   * `\u{e000}summary:[…]` first, when anything is counted: one chip per counted kind, with the
//!     kind, its emoji, the count, the words after it and the index of the first section it jumps to;
//!   * `\u{e000}section:{…}` before a section's lines and `\u{e000}/section` after them. The JSON names
//!     the kind, the section's index, the emoji, the kind's own name, the bold or `Label:` text the
//!     line started with (`label`, empty when there is none), whether that label only repeats the
//!     kind's name (`labelRepeats`), and the rest of the first line (`lead`). The lines in between
//!     are the section exactly as the agent wrote it, so a renderer that keeps the emoji draws them
//!     as they are and one that replaces it draws its own first line from the JSON and the lines
//!     after the first.
//!
//! CDXC:MarkedHandovers 2026-10-11 DECISION:
//! User: the chat colours the sections an agent marks, in one of three styles picked in Settings > Chat under Marked handovers ("Marked sections style": a thin left rule, a coloured badge in place of the emoji, or coloured labels, the default), and every style shows a summary strip at the top of the message with chips counting the marked items ("❓ 2 need you · ⚠️ 1 to read · 🧪 1 untested"; a chip jumps to its section; no strip when nothing is counted). A bold label line and the bullets under it are one section. Colours are the mockup's palette (docs/2026-10-11/chat-section-markers/index.html), In progress #00A6ED. It applies to every agent's replies that carry markers (orchestrators and their threads always mark their handovers; Marked handovers extends that to every session); copying a message still gives the agent's own text (`copyText` is the unmarked body).
//! SEE-ALSO: apps/desktop/src/app/native_chat/marked_sections.rs (GPUI drawing and the palette), apps/mobile/app/src/chat/native/transcript/markdown/MarkedSections.tsx (phone drawing), server/src/agents/marked_handovers.rs (the note that asks agents for these markers; the emoji and names must match it).

use serde_json::{json, Value};

use crate::transcript::jsstr::{js_trim, js_trim_end, js_trim_start, split_lf};
use crate::transcript::native_markdown::{fence_close, fence_open, NATIVE_MARK};

pub const NATIVE_SECTION_OPEN: &str = "\u{e000}section:";
pub const NATIVE_SECTION_CLOSE: &str = "\u{e000}/section";
pub const NATIVE_SUMMARY_OPEN: &str = "\u{e000}summary:";

/// One marker: the kind id the renderers colour by, the emoji agents write, and the kind's name.
struct Marker {
    kind: &'static str,
    emoji: &'static str,
    name: &'static str,
    /// What a summary chip says after the count (one, more than one); `None` is not counted.
    counted: Option<(&'static str, &'static str)>,
}

/// In the order the summary lists the counted kinds.
const MARKERS: [Marker; 7] = [
    Marker {
        kind: "need",
        emoji: "\u{2753}",
        name: "Needs you",
        counted: Some(("needs you", "need you")),
    },
    Marker {
        kind: "read",
        emoji: "\u{26a0}\u{fe0f}",
        name: "Read this",
        counted: Some(("to read", "to read")),
    },
    Marker {
        kind: "blocked",
        emoji: "\u{1f6d1}",
        name: "Blocked",
        counted: Some(("blocked", "blocked")),
    },
    Marker {
        kind: "untested",
        emoji: "\u{1f9ea}",
        name: "Untested",
        counted: Some(("untested", "untested")),
    },
    Marker {
        kind: "done",
        emoji: "\u{2705}",
        name: "Done",
        counted: None,
    },
    Marker {
        kind: "progress",
        emoji: "\u{1f504}",
        name: "In progress",
        counted: None,
    },
    Marker {
        kind: "shipped",
        emoji: "\u{1f680}",
        name: "Shipped",
        counted: None,
    },
];

/// The marker `text` starts with, and what follows it (a variation selector after the emoji is
/// optional: agents write ⚠ and ⚠️ alike).
fn leading_marker(text: &str) -> Option<(&'static Marker, &str)> {
    MARKERS.iter().find_map(|marker| {
        let bare = marker.emoji.trim_end_matches('\u{fe0f}');
        let rest = text.strip_prefix(bare)?;
        Some((marker, rest.strip_prefix('\u{fe0f}').unwrap_or(rest)))
    })
}

fn has_any_marker(markdown: &str) -> bool {
    MARKERS
        .iter()
        .any(|marker| markdown.contains(marker.emoji.trim_end_matches('\u{fe0f}')))
}

/// The first line of a section, taken apart.
struct Opening<'a> {
    marker: &'static Marker,
    label: String,
    lead: &'a str,
}

/// `^ {0,3}#{1,6}[ \t]+`, the heading's text when the line is one.
fn heading_text(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let hashes = rest.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let text = &rest[hashes..];
    (text.starts_with([' ', '\t'])).then(|| js_trim_start(text))
}

/// A plain `Label:` at the start of `text`: a few words with no Markdown in them before a colon.
fn plain_label(text: &str) -> Option<(&str, &str)> {
    let colon = text.find(':')?;
    let label = js_trim(&text[..colon]);
    let words = label.split_whitespace().count();
    let plain = !label.contains(['*', '`', '[', ']', '_', '(', ')', '<', '>', '!', '?', '.']);
    (plain && (1..=4).contains(&words) && label.chars().count() <= 24)
        .then(|| (label, &text[colon + 1..]))
}

/// One trailing hard break (an odd run of backslashes), which a line loses when it ends a block.
fn without_hard_break(line: &str) -> &str {
    let trimmed = line.trim_end_matches('\\');
    if (line.len() - trimmed.len()) % 2 == 1 {
        &line[..line.len() - 1]
    } else {
        line
    }
}

/// The line, read as the first line of a marked section.
///
/// A section starts on a top-level line or heading that begins with a marker, either bare
/// (`✅ **Done:** …`, `✅ Done: …`, `✅ fixed it`) or inside the bold label (`**✅ Done:** …`). A
/// list item that starts with a marker stays part of its list.
fn opening(line: &str) -> Option<Opening<'_>> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let heading = heading_text(line);
    let text = heading.unwrap_or(&line[indent..]);
    let bold_outside = text.strip_prefix("**").and_then(leading_marker);
    let (marker, after, in_bold) = match bold_outside {
        Some((marker, after)) => (marker, after, true),
        None => {
            let (marker, after) = leading_marker(text)?;
            (marker, after, false)
        }
    };
    let after = js_trim_start(after);
    let (label, lead) = if in_bold {
        match after.find("**") {
            Some(close) => (js_trim(&after[..close]).to_string(), &after[close + 2..]),
            None => (String::new(), after),
        }
    } else if let Some(bold) = after.strip_prefix("**") {
        match bold.find("**") {
            Some(close) if close > 0 => (js_trim(&bold[..close]).to_string(), &bold[close + 2..]),
            _ => (String::new(), after),
        }
    } else if let Some((label, lead)) = plain_label(after) {
        (format!("{label}:"), lead)
    } else if heading.is_some() {
        (js_trim(after).to_string(), "")
    } else {
        (String::new(), after)
    };
    // `**Done**: …` keeps its colon with the label.
    let (label, lead) = match js_trim_start(lead).strip_prefix(':') {
        Some(rest) if !label.is_empty() && !label.ends_with(':') => (format!("{label}:"), rest),
        _ => (label, lead),
    };
    Some(Opening {
        marker,
        label,
        lead: without_hard_break(js_trim_end(js_trim_start(lead))),
    })
}

/// `^( *)([-*+]|\d{1,9}[.)])( +|$)`: the indent of a list item line.
fn list_item_indent(line: &str) -> Option<usize> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    let rest = &line[indent..];
    let marker_len = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if !(1..=9).contains(&digits) || !rest[digits..].starts_with(['.', ')']) {
            return None;
        }
        digits + 1
    };
    let after = &rest[marker_len..];
    (after.is_empty() || after.starts_with(' ')).then_some(indent)
}

/// A line that opens a block the core marked (an alert, a table, a drawn fence), with the line
/// that closes it.
fn mark_region(line: &str) -> Option<String> {
    let name = line.strip_prefix(NATIVE_MARK)?;
    if name.starts_with('/') || name.starts_with("image:") {
        return None;
    }
    let name = name.split(':').next().unwrap_or(name);
    Some(format!("{NATIVE_MARK}/{name}"))
}

/// The index past the block opened at `index` (a marked region or a fence), when it opens one.
fn skip_block(lines: &[&str], index: usize) -> Option<usize> {
    let line = lines[index];
    if let Some(close) = mark_region(line) {
        let open = &close[NATIVE_MARK.len_utf8() + 1..];
        let mut depth = 1usize;
        let mut end = index + 1;
        while end < lines.len() {
            let candidate = lines[end];
            if candidate == close {
                depth -= 1;
                if depth == 0 {
                    return Some(end + 1);
                }
            } else if candidate
                .strip_prefix(NATIVE_MARK)
                .is_some_and(|name| name.split(':').next() == Some(open))
            {
                depth += 1;
            }
            end += 1;
        }
        return Some(lines.len());
    }
    let (character, run, _) = fence_open(line)?;
    let close = lines[index + 1..]
        .iter()
        .position(|line| fence_close(line).is_some_and(|(c, r)| c == character && r >= run))
        .map_or(lines.len(), |offset| index + 2 + offset);
    Some(close)
}

/// Where the section that starts at `start` ends (exclusive): its paragraph, and the list or
/// indented lines that follow it, across blank lines.
fn section_end(lines: &[&str], start: usize) -> usize {
    let mut index = start + 1;
    while index < lines.len() {
        let line = lines[index];
        if js_trim(line).is_empty() {
            let next = lines[index..]
                .iter()
                .position(|line| !js_trim(line).is_empty())
                .map(|offset| index + offset);
            let Some(next) = next else {
                return index;
            };
            let continues =
                list_item_indent(lines[next]).is_some() || lines[next].starts_with("  ");
            if !continues || opening(lines[next]).is_some() {
                return index;
            }
            index = next;
            continue;
        }
        if opening(line).is_some() || heading_text(line).is_some() || mark_region(line).is_some() {
            return index;
        }
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();
        if fence_open(line).is_some() {
            if indent < 2 {
                return index;
            }
            index = skip_block(lines, index).unwrap_or(index + 1);
            continue;
        }
        index += 1;
    }
    lines.len()
}

/// How many items a section holds: its top-level bullets, or one when it has none.
fn item_count(lines: &[&str]) -> usize {
    let indents: Vec<usize> = lines[1..]
        .iter()
        .filter_map(|line| list_item_indent(line))
        .collect();
    match indents.iter().min() {
        Some(top) => indents.iter().filter(|indent| *indent == top).count(),
        None => 1,
    }
}

/// The reply with its marked sections wrapped and its summary first. A reply without a marker is
/// returned as it is.
pub fn mark_sections(markdown: String) -> String {
    if !has_any_marker(&markdown) {
        return markdown;
    }
    let lines = split_lf(&markdown);
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 8);
    let mut counts: Vec<(&'static Marker, usize, usize)> = Vec::new();
    let mut section = 0usize;
    let mut index = 0;
    while index < lines.len() {
        if let Some(end) = skip_block(&lines, index) {
            out.extend(lines[index..end].iter().map(|line| (*line).to_string()));
            index = end;
            continue;
        }
        let Some(opening) = opening(lines[index]) else {
            out.push(lines[index].to_string());
            index += 1;
            continue;
        };
        let end = section_end(&lines, index);
        let body = &lines[index..end];
        if let Some(counted) = opening.marker.counted.map(|_| opening.marker) {
            let items = item_count(body);
            match counts
                .iter_mut()
                .find(|(marker, _, _)| marker.kind == counted.kind)
            {
                Some((_, count, _)) => *count += items,
                None => counts.push((counted, items, section)),
            }
        }
        // The line before ends its paragraph here, so its hard break would show as a backslash.
        if let Some(last) = out.last_mut() {
            *last = without_hard_break(last).to_string();
        }
        let label_name = js_trim(opening.label.trim_end_matches(':'));
        out.push(format!(
            "{NATIVE_SECTION_OPEN}{}",
            json!({
                "kind": opening.marker.kind,
                "index": section,
                "emoji": opening.marker.emoji,
                "name": opening.marker.name,
                "label": opening.label,
                "labelRepeats": label_name.eq_ignore_ascii_case(opening.marker.name),
                "lead": opening.lead,
            })
        ));
        let mut body: Vec<String> = body.iter().map(|line| (*line).to_string()).collect();
        while body.len() > 1 && js_trim(body.last().map(String::as_str).unwrap_or("")).is_empty() {
            body.pop();
        }
        if let Some(last) = body.last_mut() {
            *last = without_hard_break(last).to_string();
        }
        out.extend(body);
        out.push(NATIVE_SECTION_CLOSE.to_string());
        section += 1;
        index = end;
    }
    if section == 0 {
        return markdown;
    }
    let mut chips: Vec<Value> = Vec::new();
    for marker in MARKERS.iter() {
        let Some((_, count, first)) = counts
            .iter()
            .find(|(counted, _, _)| counted.kind == marker.kind)
        else {
            continue;
        };
        let (one, many) = marker.counted.unwrap_or(("", ""));
        chips.push(json!({
            "kind": marker.kind,
            "emoji": marker.emoji,
            "count": count,
            "text": if *count == 1 { one } else { many },
            "section": first,
        }));
    }
    if !chips.is_empty() {
        out.insert(0, format!("{NATIVE_SUMMARY_OPEN}{}", Value::Array(chips)));
    }
    out.join("\n")
}

/// The reply's text without the section and summary marks, for search: what a reader sees.
pub fn without_section_marks(text: &str) -> String {
    if !text.contains(NATIVE_SECTION_OPEN) && !text.contains(NATIVE_SUMMARY_OPEN) {
        return text.to_string();
    }
    split_lf(text)
        .into_iter()
        .filter(|line| {
            !line.starts_with(NATIVE_SECTION_OPEN)
                && !line.starts_with(NATIVE_SUMMARY_OPEN)
                && *line != NATIVE_SECTION_CLOSE
        })
        .collect::<Vec<_>>()
        .join("\n")
}
