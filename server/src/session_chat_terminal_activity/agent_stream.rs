use super::*;

/// Markers that end a Claude message block whatever their indent: the next
/// bullet, a tool gutter, the working spinner and its allowlisted cousins, the
/// thinking header, the composer prompt and the rules around it.
const CLAUDE_BLOCK_TERMINATORS: &str = "⏺⎿❯∴─╭╰│✳✶✻✽✸✹✺✷✴";

/// A row painted flush against the right edge, deeper than any wrapped prose
/// or nested list sits. Claude Code puts its own notices there (`Update
/// available! Run: …`), and one stitched itself into a streamed story.
fn is_right_aligned_notice(row: &ScreenRow, width: usize) -> bool {
    if row.indent <= CLAUDE_STATUS_CONTINUATION_MAX_INDENT {
        return false;
    }
    // Claude leaves a couple of columns of margin after the notice, and a
    // row that starts past any plausible code indentation is never prose.
    row.indent > AGENT_STREAM_MAX_ROW_INDENT || (width > 0 && row.end + 4 >= width)
}

/// Deepest indent a painted message row can have: wrapped prose sits at two,
/// nested lists and code a few dozen deeper. Anything past this is chrome.
const AGENT_STREAM_MAX_ROW_INDENT: usize = 40;

fn is_claude_block_row(row: &ScreenRow, width: usize) -> bool {
    row.indent >= CLAUDE_STATUS_CONTINUATION_INDENT
        && !is_right_aligned_notice(row, width)
        && !row
            .text
            .chars()
            .next()
            .is_some_and(|marker| CLAUDE_BLOCK_TERMINATORS.contains(marker))
}

fn agent_stream_row(row: &ScreenRow) -> AgentStreamRow {
    AgentStreamRow {
        text: row.text.clone(),
        indent: row.indent,
        after_blank: row.after_blank,
        bold: row.bold,
    }
}

/// The message block that starts at the `⏺` row `head` (or at the top of the
/// grid when the bullet has scrolled off), down to the next marker row.
pub(super) fn claude_stream_activity(
    rows: &[ScreenRow],
    head: Option<usize>,
) -> SessionChatTerminalActivity {
    let mut block = Vec::new();
    let first = match head {
        Some(head) => {
            let text = rows[head]
                .text
                .strip_prefix('⏺')
                .map_or(rows[head].text.as_str(), str::trim_start);
            block.push(AgentStreamRow {
                text: without_claude_arrow_key_hint(text).to_string(),
                indent: CLAUDE_STATUS_CONTINUATION_INDENT,
                after_blank: false,
                bold: rows[head].bold,
            });
            head + 1
        }
        None => 0,
    };
    let width = screen_width(rows);
    for row in &rows[first..] {
        if !is_claude_block_row(row, width) {
            break;
        }
        block.push(agent_stream_row(row));
    }
    if head.is_some() && block.len() == 1 {
        if let Some(tool) = claude_tool_row_without_gutter(&block[0].text) {
            return tool;
        }
    }
    let mut activity = SessionChatTerminalActivity::new(SESSION_CHAT_ACTIVITY_AGENT_STREAM, "");
    activity.stream = Some(AgentStreamRows {
        rows: block,
        head_visible: head.is_some(),
    });
    activity.render_agent_stream();
    activity
}

/// CDXC:AgentScreenDetection 2026-09-26 DECISION:
/// User: Claude's `⏺ 2 background agents launched (↓ to manage)` row shows in chat without its `(↓ to manage)` hint. The hint names a key that only works in the terminal, so the chat keeps the sentence and the agent rows under it.
fn without_claude_arrow_key_hint(text: &str) -> &str {
    text.strip_suffix(')')
        .and_then(|rest| rest.rsplit_once(" (↓ to "))
        .map_or(text, |(sentence, _)| sentence.trim_end())
}

/*
CDXC:AgentScreenDetection 2026-09-11 WHY:
Claude repaints an in-flight tool row every second with and without its
bullet and with and without its `⎿` gutter, and before the tool's input has
finished streaming it paints a `⏺ Running 1 shell command…` placeholder with
no gutter at all. Read as a message block, those rows became a streaming
bubble that blinked in and out of the chat for the whole tool run. Two shapes
prove a lone bullet row is a tool, gutter or not: the ` · 4s` clock Claude
appends only to running tool rows, and the `Running <n> …` placeholder
grammar. A row with neither still lands as a stream for at most a probe: the
transcript records the tool call the moment the row is painted, and the
client hides a stream while the newest transcript row is a tool call waiting
for its result (session-chat-terminal-stream.ts).
*/
fn claude_tool_row_without_gutter(text: &str) -> Option<SessionChatTerminalActivity> {
    let (stable, elapsed_seconds) = trailing_elapsed_status(text);
    if elapsed_seconds.is_some() && !stable.is_empty() {
        let mut activity =
            SessionChatTerminalActivity::new(SESSION_CHAT_ACTIVITY_CLAUDE_TOOL, stable);
        activity.elapsed_seconds = elapsed_seconds;
        return Some(activity);
    }
    let placeholder = text.strip_suffix('…')?;
    let rest = placeholder.strip_prefix("Running ")?;
    let (count, noun) = rest.split_once(' ')?;
    if count.is_empty()
        || !count.bytes().all(|byte| byte.is_ascii_digit())
        || noun.trim().is_empty()
    {
        return None;
    }
    Some(SessionChatTerminalActivity::new(
        SESSION_CHAT_ACTIVITY_CLAUDE_TOOL,
        text,
    ))
}

/// A grid whose top row is an indented message row with no bullet anywhere:
/// the tail of something that scrolled. Only `merge_agent_stream` can say
/// whether it continues a message already being read.
pub(super) fn headless_claude_stream_sample(
    rows: &[ScreenRow],
) -> Option<SessionChatTerminalActivity> {
    let first = rows.first()?;
    if !is_claude_block_row(first, screen_width(rows)) {
        return None;
    }
    Some(claude_stream_activity(rows, None))
}

/// Deepest relative indent a rendered stream line keeps (four opens a markdown
/// indented code block).
const AGENT_STREAM_MAX_RENDERED_INDENT: usize = 3;

/// Rows of one accumulated stream after which growth stops; a reply this tall
/// is not something the chat needs a live mirror of.
const AGENT_STREAM_MAX_ROWS: usize = 4_000;

/// Rows from the end of the accumulated stream tried as stitch anchors. The
/// last row is the tip Claude is still appending to, so it rarely matches;
/// the rows above it are complete and stable.
const AGENT_STREAM_ANCHOR_ROWS: usize = 12;

/// Shortest row text that may anchor a stitch on its own; shorter rows (a
/// lone bullet, `and`) repeat too easily to place the sample.
const AGENT_STREAM_ANCHOR_MIN_CHARS: usize = 16;

fn starts_list_item(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if matches!(first, '-' | '•' | '*' | '◦' | '▪' | '☐' | '☒' | '✔' | '✓') {
        return chars.next().is_some_and(char::is_whitespace);
    }
    if first.is_ascii_digit() {
        let rest: String = chars.collect();
        let digits_end = rest
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(rest.len());
        let after = &rest[digits_end..];
        return (after.starts_with(". ") || after.starts_with(") ")) && digits_end <= 2;
    }
    false
}

impl SessionChatTerminalActivity {
    /// `text` and `label` from `stream`: wrapped prose rows re-join into one
    /// paragraph, blank rows separate paragraphs, and rows Claude painted
    /// deeper than the message indent (list nesting, code) or that open a list
    /// item keep their own line with their relative indent.
    ///
    /// CDXC:AgentScreenDetection 2026-09-26 SEE-ALSO:
    /// A paragraph painted entirely bold is a heading or a bold-only title line (Claude draws both in bold and drops the Markdown), so it goes out as `**Title**`. The chat core holds such a title at the end of the stream until the text under it arrives (`without_trailing_section_titles` in packages/gx-chat-core/src/session/streaming.rs). The label stays plain because it is the transcript match key.
    fn render_agent_stream(&mut self) {
        let Some(stream) = self.stream.as_ref() else {
            return;
        };
        let mut lines: Vec<String> = Vec::new();
        let mut paragraph = String::new();
        let mut paragraph_bold = true;
        let mut first_paragraph: Option<String> = None;
        let flush = |paragraph: &mut String,
                     bold: &mut bool,
                     lines: &mut Vec<String>,
                     first: &mut Option<String>| {
            if paragraph.is_empty() {
                return;
            }
            if first.is_none() {
                *first = Some(paragraph.clone());
            }
            let text = std::mem::take(paragraph);
            lines.push(if *bold { format!("**{text}**") } else { text });
            *bold = true;
        };
        for row in &stream.rows {
            // Capped below four spaces: markdown reads a deeper indent as a
            // code block, and a nested list row or a wrapped tip painted deep
            // opened an empty "code" box under the streamed text.
            let relative = row
                .indent
                .saturating_sub(CLAUDE_STATUS_CONTINUATION_INDENT)
                .min(AGENT_STREAM_MAX_RENDERED_INDENT);
            let own_line = relative > 0 || starts_list_item(&row.text);
            if row.after_blank {
                flush(
                    &mut paragraph,
                    &mut paragraph_bold,
                    &mut lines,
                    &mut first_paragraph,
                );
                lines.push(String::new());
            }
            if own_line {
                flush(
                    &mut paragraph,
                    &mut paragraph_bold,
                    &mut lines,
                    &mut first_paragraph,
                );
                if first_paragraph.is_none() {
                    first_paragraph = Some(row.text.clone());
                }
                lines.push(format!("{}{}", " ".repeat(relative), row.text));
                continue;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(&row.text);
            paragraph_bold &= row.bold;
        }
        flush(
            &mut paragraph,
            &mut paragraph_bold,
            &mut lines,
            &mut first_paragraph,
        );
        let mut text = lines.join("\n");
        if !stream.head_visible {
            text.insert_str(0, "… ");
        }
        self.label = first_paragraph.unwrap_or_default();
        self.text = Some(text);
    }
}

/// Where `sample` continues `previous`: the index in each of the row the two
/// share, preferring a two-row window and accepting a single row only when it
/// occurs once in the sample.
fn agent_stream_anchor(
    previous: &[AgentStreamRow],
    sample: &[AgentStreamRow],
) -> Option<(usize, usize)> {
    let start = previous.len().saturating_sub(AGENT_STREAM_ANCHOR_ROWS);
    for index in (start..previous.len()).rev() {
        let row = &previous[index].text;
        if row.chars().count() < AGENT_STREAM_ANCHOR_MIN_CHARS {
            continue;
        }
        if index > 0 {
            let above = &previous[index - 1].text;
            if let Some(found) = sample
                .iter()
                .enumerate()
                .skip(1)
                .rev()
                .find(|(position, candidate)| {
                    candidate.text == *row && sample[position - 1].text == *above
                })
                .map(|(position, _)| position)
            {
                return Some((index, found));
            }
        }
        let mut matches = sample
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.text == *row)
            .map(|(position, _)| position);
        if let (Some(found), None) = (matches.next(), matches.next()) {
            return Some((index, found));
        }
    }
    None
}

/// Loose identity of a message's first row across repaints: Claude re-renders
/// markdown as it streams, so decoration may change while the words hold.
fn same_agent_stream_head(previous: &str, sample: &str) -> bool {
    let normalize = |text: &str| -> String {
        text.chars()
            .filter(|ch| !matches!(ch, '*' | '_' | '`' | '#' | '~'))
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let previous = normalize(previous);
    let sample = normalize(sample);
    if previous.is_empty() || sample.is_empty() {
        return previous.is_empty() && sample.is_empty();
    }
    previous.starts_with(&sample)
        || sample.starts_with(&previous)
        || previous.chars().take(20).eq(sample.chars().take(20))
}

/// Folds one `agent-stream` probe into what earlier probes accumulated for the
/// same session. Returns false when the sample cannot be placed: a headless
/// grid with nothing to stitch onto is not a message, and the caller drops it.
///
/// A sample whose bullet is on screen is complete from the message's first
/// row, so it replaces the accumulated rows outright (that is also how
/// Claude's in-place repaints reach the client). It continues the previous
/// stream when the previous rows also started at the bullet and the first rows
/// agree; otherwise it is a new message and keeps its own fresh `detected_at`.
pub fn merge_agent_stream(
    current: &mut SessionChatTerminalActivity,
    previous: Option<&SessionChatTerminalActivity>,
) -> bool {
    let Some(sample) = current.stream.take() else {
        return false;
    };
    let previous = previous.filter(|previous| previous.kind == SESSION_CHAT_ACTIVITY_AGENT_STREAM);
    let previous_rows = previous.and_then(|previous| previous.stream.as_ref());
    let merged = if sample.head_visible {
        let continues = previous_rows.is_some_and(|rows| {
            rows.head_visible
                && match (rows.rows.first(), sample.rows.first()) {
                    (Some(first), Some(head)) => same_agent_stream_head(&first.text, &head.text),
                    (None, _) => true,
                    _ => false,
                }
        });
        if continues {
            current.detected_at = previous.unwrap().detected_at.clone();
        }
        sample
    } else {
        let Some(rows) = previous_rows else {
            return false;
        };
        let Some((keep_through, resume_after)) = agent_stream_anchor(&rows.rows, &sample.rows)
        else {
            return false;
        };
        let mut stitched = rows.rows[..=keep_through].to_vec();
        if stitched.len() < AGENT_STREAM_MAX_ROWS {
            stitched.extend_from_slice(&sample.rows[resume_after + 1..]);
        }
        current.detected_at = previous.unwrap().detected_at.clone();
        AgentStreamRows {
            rows: stitched,
            head_visible: rows.head_visible,
        }
    };
    current.stream = Some(merged);
    current.render_agent_stream();
    true
}
