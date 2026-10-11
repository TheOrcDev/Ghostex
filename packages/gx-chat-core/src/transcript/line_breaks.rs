//! Which single newlines in an agent's reply or a user's prompt stay line breaks.
//!
//! Markdown joins the lines of a paragraph, and so do both renderers (the GPUI `TextView` and the
//! phone's parser in `apps/mobile/app/src/chat/native/transcript/markdown/`): a question and the
//! lettered options under it read as one run of text, and so did a multi-line prompt. The decision
//! is made here and travels inside the Markdown as a hard break, which every renderer draws as a
//! new line.
//!
//! CDXC:SessionChat 2026-09-27 DECISION:
//! User: "i dont want joined lines when agent sends stuff with newlines". Every agent's reply keeps each single newline as a line break, and so does every other place an agent's own words are drawn: thinking, side-question answers, a subagent's message card, and a message another agent sent into the session. Supersedes the 2026-09-26 decision, under which Claude, Codex, Grok Build and Cursor followed Markdown's joining except before a lettered option line (only Pi, OMP, Hermes, Antigravity and ZCode kept every newline).

use markdown::mdast::Node;
use markdown::ParseOptions;

/// How an agent's reply treats a newline inside a paragraph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentLineBreaks {
    /// Markdown's joining, except before a lettered option line.
    #[default]
    Markdown,
    /// Every newline is a line break.
    Every,
}

impl AgentLineBreaks {
    /// The rule for a session: every agent keeps its newlines (the 2026-09-27 decision above).
    pub fn for_agent(_agent: Option<&str>, _session_agent: Option<&str>) -> Self {
        Self::Every
    }
}

/// The reply with the chosen soft line breaks turned into hard breaks. Code, tables, headings and
/// HTML are untouched: only the text of paragraphs is looked at.
pub fn agent_line_breaks(markdown: &str, mode: AgentLineBreaks) -> String {
    hard_breaks(markdown, mode, "\\")
}

/// A user's prompt with every newline inside a paragraph kept as a line break: it is text somebody
/// typed, so a pasted block or a list of lines shows the way it was sent.
///
/// CDXC:SessionChat 2026-09-27 WHY: the break is two trailing spaces, not the backslash agent replies get, because the prompt then goes through the typed-path linker, whose path token runs to the next whitespace: a prompt line ending in `/Users/me/project` would become a link to `/Users/me/project\` and lose the break with it. Agent replies stay on the backslash because the phone trims trailing spaces from agent paragraph lines.
pub fn user_line_breaks(markdown: &str) -> String {
    hard_breaks(markdown, AgentLineBreaks::Every, "  ")
}

fn hard_breaks(markdown: &str, mode: AgentLineBreaks, marker: &str) -> String {
    if !markdown.contains('\n') {
        return markdown.to_string();
    }
    let Ok(tree) = markdown::to_mdast(markdown, &ParseOptions::gfm()) else {
        return markdown.to_string();
    };
    let mut offsets = Vec::new();
    collect(&tree, false, markdown, mode, &mut offsets);
    if offsets.is_empty() {
        return markdown.to_string();
    }
    offsets.sort_unstable();
    offsets.dedup();
    let mut out = String::with_capacity(markdown.len() + offsets.len() * marker.len());
    let mut cursor = 0;
    for offset in offsets {
        out.push_str(&markdown[cursor..offset]);
        out.push_str(marker);
        cursor = offset;
    }
    out.push_str(&markdown[cursor..]);
    out
}

fn collect(
    node: &Node,
    in_paragraph: bool,
    source: &str,
    mode: AgentLineBreaks,
    offsets: &mut Vec<usize>,
) {
    if in_paragraph {
        if let Node::Text(text) = node {
            if let Some(position) = &text.position {
                soft_breaks(
                    source,
                    position.start.offset,
                    position.end.offset,
                    mode,
                    offsets,
                );
            }
            return;
        }
    }
    let in_paragraph = in_paragraph || matches!(node, Node::Paragraph(_));
    for child in node.children().map(Vec::as_slice).unwrap_or_default() {
        collect(child, in_paragraph, source, mode, offsets);
    }
}

/// The insertion points, one per chosen newline in a text span: right before its line ending.
fn soft_breaks(
    source: &str,
    start: usize,
    end: usize,
    mode: AgentLineBreaks,
    offsets: &mut Vec<usize>,
) {
    let Some(span) = source.get(start..end) else {
        return;
    };
    for (index, _) in span.match_indices('\n') {
        let newline = start + index;
        let line_end = if source[..newline].ends_with('\r') {
            newline - 1
        } else {
            newline
        };
        let line = &source[..line_end];
        // Already a hard break: two trailing spaces, or an odd run of backslashes.
        let backslashes = line.len() - line.trim_end_matches('\\').len();
        if line.ends_with("  ") || backslashes % 2 == 1 {
            continue;
        }
        let next_line = source[newline + 1..].split('\n').next().unwrap_or_default();
        if mode == AgentLineBreaks::Every || starts_with_option(next_line) {
            offsets.push(line_end);
        }
    }
}

/// `A. `, `B) `, `A1. ` after indentation or a quote prefix.
fn starts_with_option(line: &str) -> bool {
    let text = line.trim_start_matches([' ', '\t', '>']);
    let mut characters = text.chars();
    if !characters
        .next()
        .is_some_and(|letter| letter.is_ascii_uppercase())
    {
        return false;
    }
    let rest = characters
        .as_str()
        .trim_start_matches(|digit: char| digit.is_ascii_digit());
    rest.strip_prefix(['.', ')'])
        .is_some_and(|after| after.starts_with([' ', '\t']))
}
